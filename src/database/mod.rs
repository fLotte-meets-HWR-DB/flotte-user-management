use postgres::{Client, Error, NoTls};
pub mod users;
use dotenv;

const DB_CONNECTION_URL: &str = "POSTGRES_CONNECTION_URL";
const DEFAULT_CONNECTION: &str = "postgres://postgres:postgres@localhost/postgrees";

/// Returns a database connection
pub fn get_connection() -> Result<Client, Error> {
    let conn_url = dotenv::var(DB_CONNECTION_URL).unwrap_or(DEFAULT_CONNECTION.to_string());
    Client::connect(conn_url.as_str(), NoTls)
}

/// Inits the database
pub fn init_database(client: &mut Client) -> Result<(), Error> {
    client.batch_execute("
        CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(255) NOT NULL,
            email           VARCHAR(255) UNIQUE NOT NULL,
            password_hash   VARCHAR(32) NOT NULL,
            salt            VARCHAR(16) NOT NULL
        );
        CREATE TABLE IF NOT EXISTS roles (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );
        CREATE TABLE IF NOT EXISTS user_roles (
            user_id         INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            PRIMARY KEY  (user_id, role_id)
        );
        CREATE TABLE IF NOT EXISTS permissions (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(128) UNIQUE NOT NULL,
            description     VARCHAR(512)
        );
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_id         INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            permission_id   INT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
            PRIMARY KEY (role_id, permission_id)
        );
    ")
}