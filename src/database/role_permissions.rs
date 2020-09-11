use crate::database::models::Permission;
use crate::database::{DatabaseClient, DatabaseError, DatabaseResult, RedisConnection, Table};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct RolePermissions {
    database_connection: Arc<Mutex<DatabaseClient>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
}

impl Table for RolePermissions {
    fn new(
        database_connection: Arc<Mutex<DatabaseClient>>,
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
            CREATE TABLE IF NOT EXISTS role_permissions (
                role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
                permission_id   INT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
                PRIMARY KEY (role_id, permission_id)
            );",
            )
            .map_err(|e| DatabaseError::Postgres(e))
    }
}

impl RolePermissions {
    pub fn by_role(&self, role_id: i32) -> DatabaseResult<Vec<Permission>> {
        let mut connection = self.database_connection.lock().unwrap();
        let rows = connection.query("SELECT * FROM role_permissions, permissions WHERE role_id = $1 AND role_permissions.permission_id = permissions.id", &[&role_id]).map_err(|e|DatabaseError::Postgres(e))?;
        serde_postgres::from_rows(&rows).map_err(|e| DatabaseError::DeserializeError(e))
    }
}
