use crate::database::permissions::Permissions;
use crate::database::role_permissions::RolePermissions;
use crate::database::roles::Roles;
use crate::database::user_roles::UserRoles;
use crate::database::users::Users;
use crate::utils::error::DatabaseResult;
use dotenv;
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

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

const DEFAULT_ADMIN_PASSWORD: &str = "flotte-admin";
const DEFAULT_ADMIN_EMAIL: &str = "admin@flotte-berlin.de";
const ENV_ADMIN_PASSWORD: &str = "ADMIN_PASSWORD";
const ENV_ADMIN_EMAIL: &str = "ADMIN_EMAIL";
const ADMIN_ROLE_NAME: &str = "SUPERADMIN";

pub trait Table {
    fn new(pool: PostgresPool) -> Self;
    fn init(&self) -> DatabaseResult<()>;
}

#[derive(Clone)]
pub struct Database {
    pool: PostgresPool,
    pub users: Users,
    pub roles: Roles,
    pub permissions: Permissions,
    pub role_permission: RolePermissions,
    pub user_roles: UserRoles,
}

impl Database {
    pub fn new() -> DatabaseResult<Self> {
        let pool = get_database_connection()?;
        Ok(Self {
            users: Users::new(PostgresPool::clone(&pool)),
            roles: Roles::new(PostgresPool::clone(&pool)),
            permissions: Permissions::new(PostgresPool::clone(&pool)),
            user_roles: UserRoles::new(PostgresPool::clone(&pool)),
            role_permission: RolePermissions::new(PostgresPool::clone(&pool)),
            pool,
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
        log::info!("Initializing role_permissions...");
        self.role_permission.init()?;

        // Create an admin user
        if let Err(e) = self.users.create_user(
            "ADMIN".to_string(),
            dotenv::var(ENV_ADMIN_EMAIL).unwrap_or(DEFAULT_ADMIN_EMAIL.to_string()),
            dotenv::var(ENV_ADMIN_PASSWORD).unwrap_or(DEFAULT_ADMIN_PASSWORD.to_string()),
        ) {
            log::debug!("Failed to create admin user {}", e);
        } else {
            log::debug!("Admin user created successfully!");
        }
        // Create an admin role where all roles get assigned to by default
        if let Err(e) = self.roles.create_role(
            ADMIN_ROLE_NAME.to_string(),
            Some("System Superadmin".to_string()),
            Vec::new(),
        ) {
            log::debug!("Failed to create admin role {}", e.to_string())
        }
        log::info!("Database fully initialized!");

        Ok(())
    }
}

pub type PostgresPool = Pool<PostgresConnectionManager<NoTls>>;

/// Returns a database connection
fn get_database_connection() -> Result<PostgresPool, r2d2::Error> {
    let conn_url = dotenv::var(DB_CONNECTION_URL).unwrap_or(DEFAULT_CONNECTION.to_string());

    Pool::new(PostgresConnectionManager::new(
        conn_url.parse().unwrap(),
        NoTls,
    ))
}
