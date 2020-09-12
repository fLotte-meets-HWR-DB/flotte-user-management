use crate::database::permissions::Permissions;
use crate::database::role_permissions::RolePermissions;
use crate::database::roles::Roles;
use crate::database::user_roles::UserRoles;
use crate::database::users::Users;
use crate::utils::error::{DBError, DatabaseClient, DatabaseResult, PostgresError};
use dotenv;
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};

pub mod database_error;
pub mod models;
pub mod permissions;
pub mod role_permissions;
pub mod roles;
pub mod tokens;
pub mod user_roles;
pub mod users;

const DB_CONNECTION_URL: &str = "POSTGRES_CONNECTION_URL";
const DEFAULT_CONNECTION: &str = "postgres://postgres:postgres@localhost/postgres";

pub trait Table {
    fn new(database_connection: Arc<Mutex<DatabaseClient>>) -> Self;
    fn init(&self) -> DatabaseResult<()>;
}

#[derive(Clone)]
pub struct Database {
    database_connection: Arc<Mutex<Client>>,
    pub users: Users,
    pub roles: Roles,
    pub permissions: Permissions,
    pub role_permission: RolePermissions,
    pub user_roles: UserRoles,
}

impl Database {
    pub fn new() -> DatabaseResult<Self> {
        let database_connection = Arc::new(Mutex::new(
            get_database_connection().map_err(|e| DBError::Postgres(e))?,
        ));
        Ok(Self {
            users: Users::new(Arc::clone(&database_connection)),
            roles: Roles::new(Arc::clone(&database_connection)),
            permissions: Permissions::new(Arc::clone(&database_connection)),
            user_roles: UserRoles::new(Arc::clone(&database_connection)),
            role_permission: RolePermissions::new(Arc::clone(&database_connection)),
            database_connection,
        })
    }

    /// Inits all database models
    pub fn init(&self) -> DatabaseResult<()> {
        log::info!("Initializing users...");
        self.users.init()?;
        log::info!("Initializing roles...");
        self.roles.init()?;
        log::info!("Initializing permissions...");
        self.permissions.init()?;
        log::info!("Initializing user_roles...");
        self.user_roles.init()?;
        log::info!("Initializing user_permissions...");
        self.role_permission.init()?;
        log::info!("Database fully initialized!");

        Ok(())
    }
}
/// Returns a database connection
fn get_database_connection() -> Result<DatabaseClient, PostgresError> {
    let conn_url = dotenv::var(DB_CONNECTION_URL).unwrap_or(DEFAULT_CONNECTION.to_string());
    Client::connect(conn_url.as_str(), NoTls)
}
