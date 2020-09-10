use crate::database::{DatabaseClient, DatabaseError, DatabaseResult, Model, RedisConnection};
use postgres::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Permissions {
    database_connection: Arc<Mutex<DatabaseClient>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
}

impl Model for Permissions {
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
            .map_err(|e| DatabaseError::Postgres(e))
    }
}
