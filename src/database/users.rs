use crate::database::user_roles::UserRoles;
use crate::database::{DatabaseError, DatabaseResult, Model, RedisConnection};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Users {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    user_roles: UserRoles,
}

impl Model for Users {
    fn new(
        database_connection: Arc<Mutex<Client>>,
        redis_connection: Arc<Mutex<RedisConnection>>,
    ) -> Self {
        Self {
            user_roles: UserRoles::new(
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
                "CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(255) NOT NULL,
            email           VARCHAR(255) UNIQUE NOT NULL,
            password_hash   VARCHAR(32) NOT NULL,
            salt            VARCHAR(16) NOT NULL
        );",
            )
            .map_err(|e| DatabaseError::Postgres(e))
    }
}
