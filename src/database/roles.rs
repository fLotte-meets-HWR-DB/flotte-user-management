//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use crate::database::models::Role;
use crate::database::role_permissions::RolePermissions;
use crate::database::{
    DatabaseResult, PostgresPool, Table, ADMIN_ROLE_NAME, DEFAULT_ADMIN_EMAIL, ENV_ADMIN_EMAIL,
};
use crate::utils::error::DBError;
use std::collections::HashSet;
use std::iter::FromIterator;

/// The role table that stores
/// all defined roles
#[derive(Clone)]
pub struct Roles {
    pool: PostgresPool,
    role_permission: RolePermissions,
}

impl Table for Roles {
    fn new(pool: PostgresPool) -> Self {
        Self {
            role_permission: RolePermissions::new(PostgresPool::clone(&pool)),
            pool,
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool.get()?.batch_execute(
            "
            CREATE TABLE IF NOT EXISTS roles (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );",
        )?;

        Ok(())
    }
}

impl Roles {
    /// Creates a new role with the given permissions
    /// that are then automatically assigned to the role
    ///
    /// The role is automatically assigned to the default admin user
    pub fn create_role(
        &self,
        name: String,
        description: Option<String>,
        permissions: Vec<i32>,
    ) -> DatabaseResult<Role> {
        let permissions: HashSet<i32> = HashSet::from_iter(permissions.into_iter());
        let mut connection = self.pool.get()?;
        let exists = connection.query_opt("SELECT id FROM roles WHERE name = $1", &[&name])?;

        if exists.is_some() {
            return Err(DBError::RecordExists);
        }

        log::trace!("Preparing transaction");
        let admin_email = dotenv::var(ENV_ADMIN_EMAIL).unwrap_or(DEFAULT_ADMIN_EMAIL.to_string());
        let mut transaction = connection.transaction()?;

        let row = transaction.query_one(
            "INSERT INTO roles (name, description) VALUES ($1, $2) RETURNING *",
            &[&name, &description],
        )?;
        let role: Role = serde_postgres::from_row(&row)?;
        for permission in permissions {
            transaction.execute(
                "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2);",
                &[&role.id, &permission],
            )?;
        }
        if let Err(e) = transaction.execute(
            "INSERT INTO user_roles (user_id, role_id) VALUES ((SELECT id FROM users WHERE email = $1), $2)",
            &[&admin_email, &role.id],
        ) {
            log::debug!("Failed to add role to admin user: {}", e);
        }

        transaction.commit()?;

        Ok(role)
    }

    /// Returns information for a role
    pub fn get_role(&self, name: String) -> DatabaseResult<Role> {
        let mut connection = self.pool.get()?;
        let result = connection.query_opt("SELECT * FROM roles WHERE roles.name = $1", &[&name])?;

        if let Some(row) = result {
            Ok(serde_postgres::from_row::<Role>(&row)?)
        } else {
            Err(DBError::RecordDoesNotExist)
        }
    }

    /// Returns a list of all roles
    pub fn get_roles(&self) -> DatabaseResult<Vec<Role>> {
        let mut connection = self.pool.get()?;
        let results = connection.query("SELECT * FROM roles", &[])?;
        let mut roles = Vec::new();

        for row in results {
            roles.push(serde_postgres::from_row::<Role>(&row)?);
        }

        Ok(roles)
    }

    pub fn update_role(
        &self,
        old_name: String,
        name: String,
        description: Option<String>,
        permissions: Vec<i32>,
    ) -> DatabaseResult<Role> {
        if old_name == ADMIN_ROLE_NAME {
            return Err(DBError::GenericError(
                "The admin role can't be altered!".to_string(),
            ));
        }
        let permissions = HashSet::from_iter(permissions.into_iter());
        let mut connection = self.pool.get()?;
        let mut transaction = connection.transaction()?;

        let id: i32 = transaction
            .query_opt("SELECT id FROM roles WHERE name = $1", &[&old_name])?
            .ok_or(DBError::RecordDoesNotExist)?
            .get(0);
        let name_exists =
            transaction.query_opt("SELECT id FROM roles WHERE name = $1", &[&name])?;
        if name_exists.is_some() {
            return Err(DBError::GenericError(format!(
                "A role with the name {} already exists!",
                name
            )));
        }
        let update_result = transaction.query_one(
            "UPDATE roles SET name = $3, description = $2 WHERE id = $1 RETURNING *",
            &[&id, &description, &name],
        )?;
        let current_permissions = transaction
            .query(
                "SELECT permission_id from role_permissions WHERE role_id = $1",
                &[&id],
            )?
            .into_iter()
            .map(|r| -> i32 { r.get(0) })
            .collect::<HashSet<i32>>();
        let new_permissions = permissions.difference(&current_permissions);
        let deleted_permissions = current_permissions.difference(&permissions);

        for new in new_permissions {
            transaction.query(
                "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2)",
                &[&id, new],
            )?;
        }
        for deleted in deleted_permissions {
            transaction.query(
                "DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2",
                &[&id, deleted],
            )?;
        }
        transaction.commit()?;

        Ok(serde_postgres::from_row::<Role>(&update_result)?)
    }

    /// Deletes a role if it exists
    pub fn delete_role(&self, name: &String) -> DatabaseResult<()> {
        if name == ADMIN_ROLE_NAME {
            return Err(DBError::GenericError(
                "The admin role can't be altered!".to_string(),
            ));
        }
        let mut connection = self.pool.get()?;
        let result = connection.query_opt("SELECT id FROM roles WHERE name = $1", &[name])?;

        if result.is_none() {
            Err(DBError::RecordDoesNotExist)
        } else {
            connection.query("DELETE FROM roles WHERE name = $1", &[name])?;

            Ok(())
        }
    }
}
