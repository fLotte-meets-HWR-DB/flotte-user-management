//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use crate::database::models::Role;
use crate::database::{DatabaseResult, PostgresPool, Table};
use crate::utils::error::DBError;
use std::collections::HashSet;
use std::iter::FromIterator;

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

    pub fn update_roles(&self, user_id: i32, roles: Vec<String>) -> DatabaseResult<Vec<Role>> {
        let mut connection = self.pool.get()?;
        let mut transaction = connection.transaction()?;
        let role_ids_result = transaction.query(
            "SELECT roles.id FROM roles WHERE roles.name = ANY ($1)",
            &[&roles],
        )?;
        let role_ids: Vec<i32> = serde_postgres::from_rows(role_ids_result.iter())?;
        let role_ids: HashSet<i32> = HashSet::from_iter(role_ids.into_iter());
        let role_result = transaction.query("SELECT roles.id FROM roles, user_roles WHERE roles.id = user_roles.role_id AND user_roles.user_id = $1", &[&user_id])?;
        let current_roles: Vec<i32> = serde_postgres::from_rows(role_result.iter())?;

        let current_roles = HashSet::from_iter(current_roles.into_iter());
        let added_roles: HashSet<&i32> = role_ids.difference(&current_roles).collect();
        let removed_roles: HashSet<&i32> = current_roles.difference(&role_ids).collect();

        for role in removed_roles {
            transaction.query(
                "DELETE FROM user_roles WHERE role_id = $1 AND user_id = $2",
                &[role, &user_id],
            )?;
        }
        for role in added_roles {
            transaction.query(
                "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)",
                &[&user_id, role],
            )?;
        }
        transaction.commit()?;

        Ok(self.by_user(user_id)?)
    }
}
