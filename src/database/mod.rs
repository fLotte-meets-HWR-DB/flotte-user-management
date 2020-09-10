use crate::database::permissions::Permissions;
use crate::database::role_permissions::RolePermissions;
use crate::database::roles::Roles;
use crate::database::user_roles::UserRoles;
use crate::database::users::Users;
use dotenv;
use postgres::{Client, NoTls};
use redis::{RedisError, RedisResult};
use std::sync::{Arc, Mutex};

pub mod models;
pub mod permissions;
pub mod role_permissions;
pub mod roles;
pub mod user_roles;
pub mod users;

const DB_CONNECTION_URL: &str = "POSTGRES_CONNECTION_URL";
const DEFAULT_CONNECTION: &str = "postgres://postgres:postgres@localhost/postgres";
const REDIS_CONNECTION_URL: &str = "REDIS_CONNECTION_URL";
const DEFAULT_REDIS_CONNECTION: &str = "redis:://127.0.0.1/";

pub type DatabaseClient = postgres::Client;
pub type RedisClient = redis::Client;
pub type RedisConnection = redis::Connection;
pub type PostgresError = postgres::Error;

pub trait Table {
    fn new(
        database_connection: Arc<Mutex<DatabaseClient>>,
        redis_connection: Arc<Mutex<RedisConnection>>,
    ) -> Self;
    fn init(&self) -> DatabaseResult<()>;
}

#[derive(Debug)]
pub enum Error {
    Redis(RedisError),
    Postgres(PostgresError),
    RecordExists,
    ScryptError,
    DeserializeError(serde_postgres::DeError),
}

pub type DatabaseError = Error;
pub type DatabaseResult<T> = Result<T, Error>;

#[derive(Clone)]
pub struct Database {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    pub users: Users,
    pub roles: Roles,
    pub permissions: Permissions,
    role_permission: RolePermissions,
    user_roles: UserRoles,
}

impl Database {
    pub fn new() -> DatabaseResult<Self> {
        let database_connection = Arc::new(Mutex::new(
            get_database_connection().map_err(|e| Error::Postgres(e))?,
        ));
        let redis_connection = Arc::new(Mutex::new(
            get_redis_connection().map_err(|e| Error::Redis(e))?,
        ));
        Ok(Self {
            users: Users::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            roles: Roles::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            permissions: Permissions::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            user_roles: UserRoles::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            role_permission: RolePermissions::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            database_connection,
            redis_connection,
        })
    }

    /// Inits all database models
    pub fn init(&self) -> DatabaseResult<()> {
        self.users.init()?;
        self.roles.init()?;
        self.permissions.init()?;
        self.user_roles.init()?;
        self.role_permission.init()?;

        Ok(())
    }
}
/// Returns a database connection
fn get_database_connection() -> Result<DatabaseClient, PostgresError> {
    let conn_url = dotenv::var(DB_CONNECTION_URL).unwrap_or(DEFAULT_CONNECTION.to_string());
    Client::connect(conn_url.as_str(), NoTls)
}

fn get_redis_connection() -> RedisResult<RedisConnection> {
    let conn_url =
        dotenv::var(REDIS_CONNECTION_URL).unwrap_or(DEFAULT_REDIS_CONNECTION.to_string());
    let client = RedisClient::open(conn_url)?;
    client.get_connection()
}
