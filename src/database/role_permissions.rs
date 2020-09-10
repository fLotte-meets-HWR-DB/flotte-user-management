use crate::database::Model;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct RolePermissions {
    connection: Arc<Mutex<Client>>,
}

impl Model for RolePermissions {
    fn new(connection: Arc<Mutex<Client>>) -> Self {
        Self { connection }
    }

    fn init(&self) -> Result<(), Error> {
        self.connection.lock().unwrap().batch_execute(
            "
            CREATE TABLE IF NOT EXISTS role_permissions (
                role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
                permission_id   INT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
                PRIMARY KEY (role_id, permission_id)
            );",
        )
    }
}
