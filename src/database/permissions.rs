//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use crate::database::models::{CreatePermissionsEntry, Permission};
use crate::database::{DatabaseResult, PostgresPool, Table, ADMIN_ROLE_NAME};
use std::collections::HashSet;
use std::iter::FromIterator;

pub(crate) const ROLE_VIEW_PERM: &str = "ROLE_VIEW";
pub(crate) const ROLE_CREATE_PERM: &str = "ROLE_CREATE";
pub(crate) const ROLE_UPDATE_PERM: &str = "ROLE_UPDATE";
pub(crate) const ROLE_DELETE_PERM: &str = "ROLE_DELETE";

pub(crate) const USER_UPDATE_PERM: &str = "USER_UPDATE";
pub(crate) const USER_VIEW_PERM: &str = "USER_VIEW";

pub(crate) const USER_MANAGEMENT_PERMISSIONS: &[(&'static str, &'static str)] = &[
    (ROLE_CREATE_PERM, "Allows the user to create roles"),
    (ROLE_UPDATE_PERM, "Allows the user to update roles"),
    (ROLE_DELETE_PERM, "Allows the user to delete roles"),
    (ROLE_VIEW_PERM, "Allows to see information of roles"),
    (
        USER_UPDATE_PERM,
        "Allows changing the name, password and email of a user",
    ),
    (USER_VIEW_PERM, "Allows to see information of users"),
];

/// The permissions table that stores defined
#[derive(Clone)]
pub struct Permissions {
    pool: PostgresPool,
}

impl Table for Permissions {
    fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool.get()?.batch_execute(
            "CREATE TABLE IF NOT EXISTS permissions (
                        id              SERIAL PRIMARY KEY,
                        name            VARCHAR(128) UNIQUE NOT NULL,
                        description     VARCHAR(512)
                    );",
        )?;

        Ok(())
    }
}

impl Permissions {
    /// Creates new permissions that are automatically assigned
    /// to the admin role upon creation
    pub fn create_permissions(
        &self,
        permissions: Vec<CreatePermissionsEntry>,
    ) -> DatabaseResult<Vec<Permission>> {
        let mut connection = self.pool.get()?;
        let mut transaction = connection.transaction()?;
        let mut created_permissions = Vec::new();

        let _: Vec<DatabaseResult<()>> = permissions
            .iter()
            .map(|CreatePermissionsEntry { name, description }| {
                let exists =
                    transaction.query_opt("SELECT * FROM permissions WHERE name = $1", &[&name])?;

                if exists.is_none() {
                    let row = transaction.query_one(
                        "INSERT INTO permissions (name, description) VALUES ($1, $2) RETURNING *;",
                        &[&name, &description],
                    )?;
                    let permission: Permission = serde_postgres::from_row(&row)?;
                    if let Err(e) = transaction.execute(
                        "INSERT INTO role_permissions (role_id, permission_id) VALUES ((SELECT id FROM roles WHERE name = $1), $2)",
                        &[&ADMIN_ROLE_NAME, &permission.id],
                    ) {
                        log::debug!(
                            "Failed to assign permission {} to ADMIN role: {}",
                            name,
                            e.to_string()
                        )
                    }

                    created_permissions.push(permission);
                } else {
                    created_permissions.push(serde_postgres::from_row(&exists.unwrap())?);
                }

                Ok(())
            })
            .collect();
        transaction.commit()?;

        Ok(created_permissions)
    }

    /// Returns a list of permission IDs that don't exist in the database
    pub fn get_not_existing(&self, permissions_vec: &Vec<i32>) -> DatabaseResult<Vec<i32>> {
        let permissions = HashSet::from_iter(permissions_vec.iter().cloned());
        let mut connection = self.pool.get()?;
        let rows = connection.query(
            "SELECT id FROM permissions WHERE id = ANY($1)",
            &[permissions_vec],
        )?;
        let existing_perms = rows
            .into_iter()
            .map(|row| -> i32 { row.get(0) })
            .collect::<HashSet<i32>>();
        let not_existing_perms = permissions
            .difference(&existing_perms)
            .cloned()
            .collect::<Vec<i32>>();

        Ok(not_existing_perms)
    }
}
