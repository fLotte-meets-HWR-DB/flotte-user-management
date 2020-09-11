use crate::database::models::Role;
use crate::database::{DatabaseResult, RedisConnection, Table};
use crate::utils::error::DBError;
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
            .map_err(DBError::from)
    }
}

impl UserRoles {
    pub fn by_user(&self, user_id: i32) -> DatabaseResult<Vec<Role>> {
        let mut connection = self.database_connection.lock().unwrap();
        let rows = connection.query(
            "SELECT * FROM user_roles, roles WHERE user_id = $1 AND roles.id = user_roles.role_id",
            &[&user_id],
        )?;

        serde_postgres::from_rows(&rows).map_err(DBError::from)
    }
}
