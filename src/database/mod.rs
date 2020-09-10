use crate::database::permissions::Permissions;
use crate::database::role_permissions::RolePermissions;
use crate::database::roles::Roles;
use crate::database::user_roles::UserRoles;
use crate::database::users::Users;
use dotenv;
use postgres::{Client, Error, NoTls};
use std::sync::{Arc, Mutex};

pub mod permissions;
pub mod role_permissions;
pub mod roles;
pub mod user_roles;
pub mod users;

const DB_CONNECTION_URL: &str = "POSTGRES_CONNECTION_URL";
const DEFAULT_CONNECTION: &str = "postgres://postgres:postgres@localhost/postgrees";

pub trait Model {
    fn new(connection: Arc<Mutex<Client>>) -> Self;
    fn init(&self) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Client>>,
    pub users: Users,
    pub roles: Roles,
    pub permissions: Permissions,
    role_permission: RolePermissions,
    user_roles: UserRoles,
}

type PostgresResult<T> = Result<T, Error>;

impl Database {
    pub fn new() -> PostgresResult<Self> {
        let connection = Arc::new(Mutex::new(get_connection()?));
        Ok(Self {
            users: Users::new(Arc::clone(&connection)),
            roles: Roles::new(Arc::clone(&connection)),
            permissions: Permissions::new(Arc::clone(&connection)),
            user_roles: UserRoles::new(Arc::clone(&connection)),
            role_permission: RolePermissions::new(Arc::clone(&connection)),
            connection,
        })
    }

    /// Inits all database models
    pub fn init(&self) -> PostgresResult<()> {
        self.users.init()?;
        self.roles.init()?;
        self.permissions.init()?;
        self.user_roles.init()?;
        self.role_permission.init()?;

        Ok(())
    }
}
/// Returns a database connection
fn get_connection() -> Result<Client, Error> {
    let conn_url = dotenv::var(DB_CONNECTION_URL).unwrap_or(DEFAULT_CONNECTION.to_string());
    Client::connect(conn_url.as_str(), NoTls)
}
