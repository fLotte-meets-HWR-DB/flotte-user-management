use redis::RedisError;
use serde_postgres::DeError;
use std::error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DBError {
    Redis(RedisError),
    Postgres(PostgresError),
    RecordExists,
    ScryptError,
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
            DBError::Redis(r) => r.to_string(),
            DBError::DeserializeError(de) => de.to_string(),
            DBError::ScryptError => "sCrypt Hash creation error".to_string(),
        }
    }
}

pub type DatabaseResult<T> = Result<T, DBError>;

impl From<PostgresError> for DBError {
    fn from(other: PostgresError) -> Self {
        Self::Postgres(other)
    }
}

impl From<RedisError> for DBError {
    fn from(other: RedisError) -> Self {
        Self::Redis(other)
    }
}

impl From<serde_postgres::DeError> for DBError {
    fn from(other: DeError) -> Self {
        Self::DeserializeError(other)
    }
}

pub type DatabaseClient = postgres::Client;
pub type RedisClient = redis::Client;
pub type RedisConnection = redis::Connection;
pub type PostgresError = postgres::Error;
