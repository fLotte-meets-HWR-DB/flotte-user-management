use crate::database::models::Role;
use crate::database::role_permissions::RolePermissions;
use crate::database::{DatabaseResult, Table};
use crate::utils::error::DBError;
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Roles {
    database_connection: Arc<Mutex<Client>>,
    role_permission: RolePermissions,
}

impl Table for Roles {
    fn new(database_connection: Arc<Mutex<Client>>) -> Self {
        Self {
            role_permission: RolePermissions::new(Arc::clone(&database_connection)),
            database_connection,
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.database_connection
            .lock()
            .unwrap()
            .batch_execute(
                "
            CREATE TABLE IF NOT EXISTS roles (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );",
            )
            .map_err(DBError::from)
    }
}

impl Roles {
    pub fn create_role(
        &self,
        name: String,
        description: Option<String>,
        permissions: Vec<i32>,
    ) -> DatabaseResult<Role> {
        let mut connection = self.database_connection.lock().unwrap();
        let exists = connection.query_opt("SELECT id FROM roles WHERE name = $1", &[&name])?;

        if exists.is_some() {
            return Err(DBError::RecordExists);
        }
        log::trace!("Preparing transaction");
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
                "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)",
                &[&1, &role.id],
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
}
