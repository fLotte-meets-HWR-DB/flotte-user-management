use crate::database::models::Role;
use crate::database::role_permissions::RolePermissions;
use crate::database::{DatabaseResult, RedisConnection, Table};
use crate::utils::error::DBError;
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Roles {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    role_permission: RolePermissions,
}

impl Table for Roles {
    fn new(
        database_connection: Arc<Mutex<Client>>,
        redis_connection: Arc<Mutex<RedisConnection>>,
    ) -> Self {
        Self {
            role_permission: RolePermissions::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            database_connection,
            redis_connection,
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

            Ok(role)
        };
        if let Err(_) = result {
            log::trace!("Rollback");
            transaction.rollback()?;
        } else {
            log::trace!("Commit");
            transaction.commit()?;
        }

        result
    }
}
