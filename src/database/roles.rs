use crate::database::role_permissions::RolePermissions;
use crate::database::Model;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Roles {
    connection: Arc<Mutex<Client>>,
    role_permission: RolePermissions,
}

impl Model for Roles {
    fn new(connection: Arc<Mutex<Client>>) -> Self {
        Self {
            role_permission: RolePermissions::new(Arc::clone(&connection)),
            connection,
        }
    }

    fn init(&self) -> Result<(), Error> {
        self.connection.lock().unwrap().batch_execute(
            "
            CREATE TABLE IF NOT EXISTS roles (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );",
        )
    }
}
