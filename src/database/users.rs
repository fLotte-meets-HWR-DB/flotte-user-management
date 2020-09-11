use crate::database::models::UserRecord;
use crate::database::tokens::SessionTokens;
use crate::database::user_roles::UserRoles;
use crate::database::{DatabaseResult, RedisConnection, Table};
use crate::utils::error::DBError;
use crate::utils::{create_salt, get_user_id_from_token, TOKEN_LENGTH};
use postgres::Client;
use scrypt::ScryptParams;
use std::sync::{Arc, Mutex};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone)]
pub struct Users {
    database_connection: Arc<Mutex<Client>>,
    redis_connection: Arc<Mutex<RedisConnection>>,
    user_roles: UserRoles,
}

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
            .map_err(DBError::from)
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
        let mut password = Zeroizing::new(password);

        if !connection
            .query("SELECT email FROM users WHERE email = $1", &[&email])?
            .is_empty()
        {
            return Err(DBError::RecordExists);
        }
        let salt = Zeroizing::new(create_salt());
        let mut pw_hash = Zeroizing::new([0u8; 32]);
        scrypt::scrypt(
            password.as_bytes(),
            &*salt,
            &ScryptParams::recommended(),
            &mut *pw_hash,
        )
        .map_err(|_| DBError::ScryptError)?;
        password.zeroize();
        let row = connection.query_one("
            INSERT INTO users (name, email, password_hash, salt) VALUES ($1, $2, $3, $4) RETURNING *;
        ", &[&name, &email, &pw_hash.to_vec(), &salt.to_vec()])?;

        Ok(UserRecord::from_ordered_row(&row))
    }

    pub fn create_token(&self, email: String, password: String) -> DatabaseResult<SessionTokens> {
        if self.validate_login(&email, password)? {
            let mut connection = self.database_connection.lock().unwrap();
            let row = connection.query_one("SELECT id FROM users WHERE email = $1", &[&email])?;
            let id: i32 = row.get(0);
            let mut redis_connection = self.redis_connection.lock().unwrap();

            let tokens = SessionTokens::new(id);
            tokens.store(&mut redis_connection)?;

            Ok(tokens)
        } else {
            Err(DBError::GenericError("Invalid password".to_string()))
        }
    }

    pub fn validate_request_token(
        &self,
        token: &[u8; TOKEN_LENGTH],
    ) -> DatabaseResult<(bool, i32)> {
        let id = get_user_id_from_token(token);
        let mut redis_connection = self.redis_connection.lock().unwrap();
        let tokens = SessionTokens::retrieve(id, &mut redis_connection)?;

        Ok((tokens.request_token == *token, tokens.request_ttl))
    }

    pub fn validate_refresh_token(
        &self,
        token: &[u8; TOKEN_LENGTH],
    ) -> DatabaseResult<(bool, i32)> {
        let id = get_user_id_from_token(token);
        let mut redis_connection = self.redis_connection.lock().unwrap();
        let tokens = SessionTokens::retrieve(id, &mut redis_connection)?;

        Ok((tokens.refresh_token == *token, tokens.refresh_ttl))
    }

    pub fn refresh_tokens(
        &self,
        refresh_token: &[u8; TOKEN_LENGTH],
    ) -> DatabaseResult<SessionTokens> {
        let id = get_user_id_from_token(refresh_token);
        let mut redis_connection = self.redis_connection.lock().unwrap();
        let mut tokens = SessionTokens::retrieve(id, &mut redis_connection)?;

        if tokens.refresh_token == *refresh_token {
            tokens.refresh();
            tokens.store(&mut redis_connection)?;

            Ok(tokens)
        } else {
            Err(DBError::GenericError("Invalid refresh token!".to_string()))
        }
    }

    fn validate_login(&self, email: &String, password: String) -> DatabaseResult<bool> {
        let password = Zeroizing::new(password);
        let mut connection = self.database_connection.lock().unwrap();
        let row = connection.query_one(
            "SELECT password_hash, salt FROM users WHERE email = $1",
            &[&email],
        )?;
        let original_pw_hash: Zeroizing<Vec<u8>> = Zeroizing::new(row.get(0));
        let salt: Zeroizing<Vec<u8>> = Zeroizing::new(row.get(1));
        let mut pw_hash = Zeroizing::new([0u8; 32]);

        scrypt::scrypt(
            password.as_bytes(),
            &*salt,
            &ScryptParams::recommended(),
            &mut *pw_hash,
        )
        .map_err(|_| DBError::ScryptError)?;

        Ok(*pw_hash == *original_pw_hash.as_slice())
    }
}
