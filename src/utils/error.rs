use serde_postgres::DeError;
use std::error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DBError {
    Postgres(PostgresError),
    RecordExists,
    BCryptError,
    DeserializeError(serde_postgres::DeError),
    GenericError(String),
}

impl Display for DBError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl error::Error for DBError {}

impl DBError {
    pub fn to_string(&self) -> String {
        match self {
            DBError::GenericError(g) => g.clone(),
            DBError::RecordExists => "Record Exists".to_string(),
            DBError::Postgres(p) => p.to_string(),
            DBError::DeserializeError(de) => de.to_string(),
            DBError::BCryptError => "BCrypt Hash creation error".to_string(),
        }
    }
}

pub type DatabaseResult<T> = Result<T, DBError>;

impl From<PostgresError> for DBError {
    fn from(other: PostgresError) -> Self {
        Self::Postgres(other)
    }
}

impl From<serde_postgres::DeError> for DBError {
    fn from(other: DeError) -> Self {
        Self::DeserializeError(other)
    }
}

impl From<String> for DBError {
    fn from(other: String) -> Self {
        Self::GenericError(other)
    }
}

pub type DatabaseClient = postgres::Client;
pub type PostgresError = postgres::Error;