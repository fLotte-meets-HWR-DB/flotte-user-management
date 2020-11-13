use crate::database::models::{CreatePermissionsEntry, Permission};
use crate::utils::error::DBError;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use zeroize::Zeroize;

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

#[derive(Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<i32>,
}

#[derive(Deserialize)]
pub struct CreatePermissionsRequest {
    pub permissions: Vec<CreatePermissionsEntry>,
}

#[derive(Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct LoginMessage {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct RefreshMessage {
    pub refresh_token: String,
}

#[derive(Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct LogoutMessage {
    pub request_token: String,
}

#[derive(Serialize)]
pub struct LogoutConfirmation {
    pub success: bool,
}

#[derive(Serialize)]
pub struct FullRowData {
    pub id: i32,
    pub name: String,
    pub permissions: Vec<Permission>,
}
