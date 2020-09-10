use crate::database::Model;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Permissions {
    connection: Arc<Mutex<Client>>,
}

impl Model for Permissions {
    fn new(connection: Arc<Mutex<Client>>) -> Self {
        Self { connection }
    }

    fn init(&self) -> Result<(), Error> {
        self.connection.lock().unwrap().batch_execute(
            "CREATE TABLE IF NOT EXISTS permissions (
                        id              SERIAL PRIMARY KEY,
                        name            VARCHAR(128) UNIQUE NOT NULL,
                        description     VARCHAR(512)
                    );",
        )
    }
}
