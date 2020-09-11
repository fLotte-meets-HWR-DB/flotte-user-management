use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[derive(Deserialize)]
pub struct ValidateTokenRequest {
    pub token: [u8; 32],
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

#[derive(Serialize)]
pub struct InfoEntry {
    name: String,
    method: [u8; 4],
    description: String,
    data: String,
}

impl InfoEntry {
    pub fn new(name: &str, method: [u8; 4], description: &str, data: &str) -> Self {
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
    pub role_ids: Vec<i32>,
}
