use crate::database::models::{CreatePermissionsEntry, Permission};
use crate::database::{DatabaseResult, PostgresPool, Table, ADMIN_ROLE_NAME};

pub(crate) const CREATE_ROLE_PERMISSION: &str = "ROLE_CREATE";
pub(crate) const UPDATE_ROLE_PERMISSION: &str = "ROLE_UPDATE";
pub(crate) const DELETE_ROLE_PERMISSION: &str = "ROLE_DELETE";
pub(crate) const DEFAULT_PERMISSIONS: &[(&'static str, &'static str)] = &[
    (CREATE_ROLE_PERMISSION, "Allows the user to create roles"),
    (UPDATE_ROLE_PERMISSION, "Allows the user to update roles"),
    (DELETE_ROLE_PERMISSION, "Allows the user to delete roles"),
];

/// The permissions table that stores defined
#[derive(Clone)]
pub struct Permissions {
    pool: PostgresPool,
}

impl Table for Permissions {
    fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool.get()?.batch_execute(
            "CREATE TABLE IF NOT EXISTS permissions (
                        id              SERIAL PRIMARY KEY,
                        name            VARCHAR(128) UNIQUE NOT NULL,
                        description     VARCHAR(512)
                    );",
        )?;

        Ok(())
    }
}

impl Permissions {
    /// Creates new permissions that are automatically assigned
    /// to the admin role upon creation
    pub fn create_permissions(
        &self,
        permissions: Vec<CreatePermissionsEntry>,
    ) -> DatabaseResult<Vec<Permission>> {
        let mut connection = self.pool.get()?;
        let mut transaction = connection.transaction()?;
        let mut created_permissions = Vec::new();

        let _: Vec<DatabaseResult<()>> = permissions
            .iter()
            .map(|CreatePermissionsEntry { name, description }| {
                let exists =
                    transaction.query_opt("SELECT * FROM permissions WHERE name = $1", &[&name])?;

                if exists.is_none() {
                    let row = transaction.query_one(
                        "INSERT INTO permissions (name, description) VALUES ($1, $2) RETURNING *;",
                        &[&name, &description],
                    )?;
                    let permission: Permission = serde_postgres::from_row(&row)?;
                    if let Err(e) = transaction.execute(
                        "INSERT INTO role_permissions (role_id, permission_id) VALUES ((SELECT id FROM roles WHERE name = $1), $2)",
                        &[&ADMIN_ROLE_NAME, &permission.id],
                    ) {
                        log::debug!(
                            "Failed to assign permission {} to ADMIN role: {}",
                            name,
                            e.to_string()
                        )
                    }

                    created_permissions.push(permission);
                } else {
                    created_permissions.push(serde_postgres::from_row(&exists.unwrap())?);
                }

                Ok(())
            })
            .collect();
        transaction.commit()?;

        Ok(created_permissions)
    }
}
