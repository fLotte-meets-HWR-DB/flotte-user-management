use crate::database::models::Role;
use crate::database::role_permissions::RolePermissions;
use crate::database::{DatabaseResult, PostgresPool, Table, DEFAULT_ADMIN_EMAIL, ENV_ADMIN_EMAIL};
use crate::utils::error::DBError;

/// The role table that stores
/// all defined roles
#[derive(Clone)]
pub struct Roles {
    pool: PostgresPool,
    role_permission: RolePermissions,
}

impl Table for Roles {
    fn new(pool: PostgresPool) -> Self {
        Self {
            role_permission: RolePermissions::new(PostgresPool::clone(&pool)),
            pool,
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool.get()?.batch_execute(
            "
            CREATE TABLE IF NOT EXISTS roles (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );",
        )?;

        Ok(())
    }
}

impl Roles {
    /// Creates a new role with the given permissions
    /// that are then automatically assigned to the role
    ///
    /// The role is automatically assigned to the default admin user
    pub fn create_role(
        &self,
        name: String,
        description: Option<String>,
        permissions: Vec<i32>,
    ) -> DatabaseResult<Role> {
        let mut connection = self.pool.get()?;
        let exists = connection.query_opt("SELECT id FROM roles WHERE name = $1", &[&name])?;

        if exists.is_some() {
            return Err(DBError::RecordExists);
        }
        let permissions_exist = connection.query(
            "SELECT id FROM permissions WHERE permissions.id = ANY ($1)",
            &[&permissions],
        )?;
        if permissions_exist.len() != permissions.len() {
            return Err(DBError::GenericError(format!(
                "Not all provided permissions exist! Existing permissions: {:?}",
                permissions_exist
                    .iter()
                    .map(|row| -> i32 { row.get(0) })
                    .collect::<Vec<i32>>()
            )));
        }

        log::trace!("Preparing transaction");
        let admin_email = dotenv::var(ENV_ADMIN_EMAIL).unwrap_or(DEFAULT_ADMIN_EMAIL.to_string());
        let mut transaction = connection.transaction()?;

        let result: DatabaseResult<Role> = {
            let row = transaction.query_one(
                "INSERT INTO roles (name, description) VALUES ($1, $2) RETURNING *",
                &[&name, &description],
            )?;
            let role: Role = serde_postgres::from_row(&row)?;
            for permission in permissions {
                transaction.execute(
                    "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2);",
                    &[&role.id, &permission],
                )?;
            }
            if let Err(e) = transaction.execute(
                "INSERT INTO user_roles (user_id, role_id) VALUES ((SELECT id FROM users WHERE email = $1), $2)",
                &[&admin_email, &role.id],
            ) {
                log::debug!("Failed to add role to admin user: {}", e);
            }

            Ok(role)
        };
        match result {
            Err(e) => {
                log::warn!("Failed to create role {}: {}", name, e);
                log::trace!("Rolling back...");
                transaction.rollback()?;
                log::trace!("Rolled back!");
                Err(e)
            }
            Ok(role) => {
                log::debug!("Successfully created role {} with id {}", name, role.id);
                log::trace!("Committing...");
                transaction.commit()?;
                log::trace!("Committed!");

                Ok(role)
            }
        }
    }

    /// Returns information for a role
    pub fn get_role(&self, name: String) -> DatabaseResult<Role> {
        let mut connection = self.pool.get()?;
        let result = connection.query_opt("SELECT * FROM roles WHERE roles.name = $1", &[&name])?;

        if let Some(row) = result {
            Ok(serde_postgres::from_row::<Role>(&row)?)
        } else {
            Err(DBError::RecordDoesNotExist)
        }
    }
}
