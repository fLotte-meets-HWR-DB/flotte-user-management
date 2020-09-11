use crate::database::models::UserRecord;
use crate::database::tokens::{SessionTokens, TokenStore};
use crate::database::user_roles::UserRoles;
use crate::database::{DatabaseResult, Table};
use crate::utils::error::DBError;
use crate::utils::{create_salt, hash_password};

use postgres::Client;
use std::sync::{Arc, Mutex};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone)]
pub struct Users {
    database_connection: Arc<Mutex<Client>>,
    user_roles: UserRoles,
    token_store: Arc<Mutex<TokenStore>>,
}

impl Table for Users {
    fn new(database_connection: Arc<Mutex<Client>>) -> Self {
        Self {
            user_roles: UserRoles::new(Arc::clone(&database_connection)),
            database_connection,
            token_store: Arc::new(Mutex::new(TokenStore::new())),
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
        let pw_hash =
            hash_password(password.as_bytes(), &*salt).map_err(|e| DBError::GenericError(e))?;
        password.zeroize();
        let row = connection.query_one("
            INSERT INTO users (name, email, password_hash, salt) VALUES ($1, $2, $3, $4) RETURNING *;
        ", &[&name, &email, &pw_hash.to_vec(), &salt.to_vec()])?;

        Ok(UserRecord::from_ordered_row(&row))
    }

    pub fn create_tokens(
        &self,
        email: &String,
        password: &String,
    ) -> DatabaseResult<SessionTokens> {
        if self.validate_login(&email, password)? {
            let mut connection = self.database_connection.lock().unwrap();
            let row = connection.query_one("SELECT id FROM users WHERE email = $1", &[&email])?;
            let id: i32 = row.get(0);

            let tokens = SessionTokens::new(id);
            tokens.store(&mut self.token_store.lock().unwrap())?;

            Ok(tokens)
        } else {
            Err(DBError::GenericError("Invalid password".to_string()))
        }
    }

    pub fn validate_request_token(&self, token: &String) -> DatabaseResult<(bool, i32)> {
        let store = self.token_store.lock().unwrap();
        let entry = store.get_request_token(&token);

        if let Some(entry) = entry {
            Ok((true, entry.request_ttl()))
        } else {
            Ok((false, -1))
        }
    }

    pub fn validate_refresh_token(&self, token: &String) -> DatabaseResult<(bool, i32)> {
        let store = self.token_store.lock().unwrap();
        let entry = store.get_refresh_token(&token);

        if let Some(entry) = entry {
            Ok((true, entry.refresh_ttl()))
        } else {
            Ok((false, -1))
        }
    }

    pub fn refresh_tokens(&self, refresh_token: &String) -> DatabaseResult<SessionTokens> {
        let mut token_store = self.token_store.lock().unwrap();
        let tokens = token_store.get_refresh_token(refresh_token);
        if let Some(mut tokens) = tokens.and_then(|t| SessionTokens::from_entry(t)) {
            tokens.refresh();
            tokens.store(&mut token_store)?;

            Ok(tokens)
        } else {
            Err(DBError::GenericError("Invalid refresh token!".to_string()))
        }
    }

    fn validate_login(&self, email: &String, password: &String) -> DatabaseResult<bool> {
        let mut connection = self.database_connection.lock().unwrap();
        let row = connection
            .query_opt(
                "SELECT password_hash, salt FROM users WHERE email = $1",
                &[&email],
            )?
            .ok_or(DBError::GenericError(format!(
                "No user with the email '{}' found",
                &email
            )))?;
        let original_pw_hash: Zeroizing<Vec<u8>> = Zeroizing::new(row.get(0));
        let salt: Zeroizing<Vec<u8>> = Zeroizing::new(row.get(1));
        let pw_hash =
            hash_password(password.as_bytes(), &*salt).map_err(|e| DBError::GenericError(e))?;

        Ok(pw_hash == *original_pw_hash.as_slice())
    }
}
