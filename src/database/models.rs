//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use postgres::Row;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Record to store data in when retrieving rows from the users table
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Vec<u8>,
    pub salt: Vec<u8>,
    pub attributes: serde_json::Value,
}

impl UserRecord {
    pub fn from_row(row: Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            salt: row.get("salt"),
            attributes: row.get("attributes"),
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

/// Information about the user that doesn't contain any critical information
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UserInformation {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub attributes: Value,
}

impl UserInformation {
    pub fn from_row(row: Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            attributes: row.get("attributes"),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UserFullInformation {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub attributes: Value,
    pub roles: Vec<Role>,
}

impl From<UserRecord> for UserInformation {
    fn from(record: UserRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            email: record.email,
            attributes: record.attributes,
        }
    }
}
