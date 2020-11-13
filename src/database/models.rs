//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use postgres::Row;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Record to store data in when retrieving rows from the users table
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct UserRecord {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Vec<u8>,
    pub salt: Vec<u8>,
}

impl UserRecord {
    pub fn from_ordered_row(row: &Row) -> Self {
        Self {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            password_hash: row.get(3),
            salt: row.get(4),
        }
    }
}

/// A row of the permission table that can be serialized and sent
/// via the rcp connection
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Permission {
    pub id: i32,
    pub name: String,
    pub description: String,
}

/// A row of the role table that can be serialized and sent
/// via the rcp connection
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Role {
    pub id: i32,
    pub name: String,
    pub description: String,
}

/// A CreatePermissionEntry data structure that is used as an argument for the
/// bulk permission creation function of the Users Model and can directly be deserialized
/// from the corresponding rcp message.
#[derive(Deserialize)]
pub struct CreatePermissionsEntry {
    pub name: String,
    pub description: String,
}
