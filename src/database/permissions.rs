use crate::database::models::{CreatePermissionsEntry, Permission};
use crate::database::{DatabaseClient, DatabaseResult, RedisConnection, Table};
use crate::utils::error::DBError;
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Permissions {
    database_connection: Arc<Mutex<DatabaseClient>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
}

impl Table for Permissions {
    fn new(
        database_connection: Arc<Mutex<Client>>,
        redis_connection: Arc<Mutex<RedisConnection>>,
    ) -> Self {
        Self {
            database_connection,
            redis_connection,
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.database_connection
            .lock()
            .unwrap()
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
        let mut connection = self.database_connection.lock().unwrap();
        let mut transaction = connection.transaction()?;
        let mut created_permissions = Vec::new();
        let _: Vec<DatabaseResult<()>> = permissions
            .iter()
            .map(|CreatePermissionsEntry { name, description }| {
                let exists = transaction
                    .query_opt("SELECT id FROM permissions WHERE name = $1", &[&name])?;

                if exists.is_none() {
                    let row = transaction.query_one(
                        "INSERT INTO permissions (name, description) VALUES ($1, $2) RETURNING *;",
                        &[&name, &description],
                    )?;

                    created_permissions.push(serde_postgres::from_row(&row)?);
                }

                Ok(())
            })
            .collect();
        transaction.commit()?;
        Ok(created_permissions)
    }
}
