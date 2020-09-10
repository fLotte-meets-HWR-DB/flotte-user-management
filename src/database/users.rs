use crate::database::user_roles::UserRoles;
use crate::database::Model;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Users {
    connection: Arc<Mutex<Client>>,
    user_roles: UserRoles,
}

impl Model for Users {
    fn new(connection: Arc<Mutex<Client>>) -> Self {
        Self {
            user_roles: UserRoles::new(Arc::clone(&connection)),
            connection,
        }
    }

    fn init(&self) -> Result<(), Error> {
        self.connection.lock().unwrap().batch_execute(
            "CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(255) NOT NULL,
            email           VARCHAR(255) UNIQUE NOT NULL,
            password_hash   VARCHAR(32) NOT NULL,
            salt            VARCHAR(16) NOT NULL
        );",
        )
    }
}
