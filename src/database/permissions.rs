use crate::database::models::{CreatePermissionsEntry, Permission};
use crate::database::{DatabaseResult, PostgresPool, Table};
use crate::utils::error::DBError;

#[derive(Clone)]
pub struct Permissions {
    pool: PostgresPool,
}

impl Table for Permissions {
    fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool
            .get()?
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS permissions (
                        id              SERIAL PRIMARY KEY,
                        name            VARCHAR(128) UNIQUE NOT NULL,
                        description     VARCHAR(512)
                    );",
            )
            .map_err(DBError::from)
    }
}

impl Permissions {
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

                    created_permissions.push(serde_postgres::from_row(&row)?);
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
