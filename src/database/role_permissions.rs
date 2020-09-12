use crate::database::models::Permission;
use crate::database::{DatabaseResult, PostgresPool, Table};
use crate::utils::error::DBError;

/// The m-n connection table for
/// roles and permissions
#[derive(Clone)]
pub struct RolePermissions {
    pool: PostgresPool,
}

impl Table for RolePermissions {
    fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool
            .get()?
            .batch_execute(
                "
            CREATE TABLE IF NOT EXISTS role_permissions (
                role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
                permission_id   INT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
                PRIMARY KEY (role_id, permission_id)
            );",
            )
            .map_err(DBError::from)
    }
}

impl RolePermissions {
    /// Returns all permissions for a role
    pub fn by_role(&self, role_id: i32) -> DatabaseResult<Vec<Permission>> {
        let mut connection = self.pool.get()?;
        let rows = connection.query(
            "SELECT * FROM role_permissions, permissions WHERE role_id = $1 AND role_permissions.permission_id = permissions.id", 
            &[&role_id])?;

        serde_postgres::from_rows(&rows).map_err(DBError::from)
    }
}
