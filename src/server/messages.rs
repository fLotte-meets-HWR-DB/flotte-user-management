//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use std::error::Error;
use std::fmt;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use zeroize::Zeroize;

use crate::database::models::{CreatePermissionsEntry, Permission, UserFullInformation};
use crate::utils::error::DBError;
use serde_json::Value;

#[derive(Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorMessage {
    message: String,
}

impl ErrorMessage {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ErrorMessage {}

impl From<DBError> for ErrorMessage {
    fn from(other: DBError) -> Self {
        Self::new(other.to_string())
    }
}

#[derive(Serialize)]
pub struct InfoEntry {
    name: String,
    method: String,
    description: String,
    data: String,
}

impl InfoEntry {
    pub fn new(name: &str, method: [u8; 4], description: &str, data: &str) -> Self {
        let method = format!(
            "0x{:x} 0x{:x} 0x{:x} 0x{:x}",
            method[0], method[1], method[2], method[3]
        );
        Self {
            method,
            name: name.to_string(),
            description: description.to_string(),
            data: data.to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct GetPermissionsRequest {
    pub roles: Vec<i32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ModifyRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<i32>,
}

#[derive(Deserialize)]
pub struct CreatePermissionsRequest {
    pub permissions: Vec<CreatePermissionsEntry>,
}

#[derive(Deserialize, Zeroize, JsonSchema)]
#[zeroize(drop)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct LoginResponse {
    pub request_token: String,
    pub refresh_token: String,
    pub request_ttl: i32,
    pub refresh_ttl: i32,
    pub user: UserFullInformation,
}

#[derive(Deserialize, Zeroize, JsonSchema)]
#[zeroize(drop)]
pub struct RefreshMessage {
    pub refresh_token: String,
}

#[derive(Deserialize, Zeroize, JsonSchema)]
#[zeroize(drop)]
pub struct LogoutMessage {
    pub request_token: String,
}

#[derive(Serialize, JsonSchema)]
pub struct LogoutConfirmation {
    pub success: bool,
}

#[derive(Serialize, JsonSchema)]
pub struct FullRoleData {
    pub id: i32,
    pub name: String,
    pub permissions: Vec<Permission>,
}

#[derive(Serialize, JsonSchema)]
pub struct DeleteRoleResponse {
    pub success: bool,
    pub role: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub roles: Option<Vec<String>>,
    pub attributes: Option<Value>,
    pub own_password: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub attributes: Value,
}

#[derive(Deserialize, JsonSchema, Zeroize)]
#[zeroize(drop)]
pub struct DeleteUserRequest {
    pub own_password: String,
}

#[derive(Serialize, JsonSchema)]
pub struct DeleteUserResponse {
    pub email: String,
    pub success: bool,
}
