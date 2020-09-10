use crate::database::role_permissions::RolePermissions;
use crate::database::{DatabaseError, DatabaseResult, Model, RedisConnection};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Roles {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    role_permission: RolePermissions,
}

impl Model for Roles {
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
            .map_err(|e| DatabaseError::Postgres(e))
    }
}
