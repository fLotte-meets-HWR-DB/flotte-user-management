use crate::database::{DatabaseError, DatabaseResult, RedisConnection, Table};
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UserRoles {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
}

impl Table for UserRoles {
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
                "
        CREATE TABLE IF NOT EXISTS user_roles (
            user_id         INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            PRIMARY KEY  (user_id, role_id)
        );",
            )
            .map_err(|e| DatabaseError::Postgres(e))
    }
}
