use crate::database::models::UserRecord;
use crate::database::user_roles::UserRoles;
use crate::database::{DatabaseError, DatabaseResult, RedisConnection, Table};
use crate::utils::create_salt;
use postgres::{Client, Error};
use scrypt::ScryptParams;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Users {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    user_roles: UserRoles,
}

const SALT_LENGTH: usize = 16;

impl Table for Users {
    fn new(
        database_connection: Arc<Mutex<Client>>,
        redis_connection: Arc<Mutex<RedisConnection>>,
    ) -> Self {
        Self {
            user_roles: UserRoles::new(
                Arc::clone(&database_connection),
                Arc::clone(&redis_connection),
            ),
            database_connection,
            redis_connection,
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.database_connection
            .lock()
            .unwrap()
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(255) NOT NULL,
            email           VARCHAR(255) UNIQUE NOT NULL,
            password_hash   BYTEA NOT NULL,
            salt            BYTEA NOT NULL
        );",
            )
            .map_err(|e| DatabaseError::Postgres(e))
    }
}

impl Users {
    pub fn create_user(
        &self,
        name: String,
        email: String,
        password: String,
    ) -> DatabaseResult<UserRecord> {
        let mut connection = self.database_connection.lock().unwrap();

        if !connection
            .query("SELECT email FROM users WHERE email = $1", &[&email])
            .map_err(|e| DatabaseError::Postgres(e))?
            .is_empty()
        {
            return Err(DatabaseError::RecordExists);
        }
        let salt = create_salt(SALT_LENGTH);
        let mut pw_hash = [0u8; 32];
        scrypt::scrypt(
            password.as_bytes(),
            &salt,
            &ScryptParams::recommended(),
            &mut pw_hash,
        )
        .map_err(|_| DatabaseError::ScryptError)?;
        let row = connection.query_one("
            INSERT INTO users (name, email, password_hash, salt) VALUES ($1, $2, $3, $4) RETURNING *;
        ", &[&name, &email, &pw_hash.to_vec(), &salt.to_vec()]).map_err(|e|DatabaseError::Postgres(e))?;

        Ok(UserRecord::from_ordered_row(&row))
    }
}
