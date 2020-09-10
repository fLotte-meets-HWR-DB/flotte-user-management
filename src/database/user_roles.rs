use crate::database::Model;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UserRoles {
    connection: Arc<Mutex<Client>>,
}

impl Model for UserRoles {
    fn new(connection: Arc<Mutex<Client>>) -> Self {
        Self { connection }
    }

    fn init(&self) -> Result<(), Error> {
        self.connection.lock().unwrap().batch_execute(
            "
        CREATE TABLE IF NOT EXISTS user_roles (
            user_id         INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            PRIMARY KEY  (user_id, role_id)
        );",
        )
    }
}
