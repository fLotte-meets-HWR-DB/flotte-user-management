use crate::database::models::Role;
use crate::database::{DatabaseResult, PostgresPool, Table};
use crate::utils::error::DBError;

/// A table that stores the relation between users and roles
#[derive(Clone)]
pub struct UserRoles {
    pool: PostgresPool,
}

impl Table for UserRoles {
    fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool
            .get()?
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
    /// Returns all roles a user is asigned to
    pub fn by_user(&self, user_id: i32) -> DatabaseResult<Vec<Role>> {
        let mut connection = self.pool.get()?;
        let rows = connection.query(
            "SELECT * FROM user_roles, roles WHERE user_id = $1 AND roles.id = user_roles.role_id",
            &[&user_id],
        )?;

        serde_postgres::from_rows(&rows).map_err(DBError::from)
    }
}
