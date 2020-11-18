//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use std::sync::Arc;

use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::database::models::{Permission, UserInformation, UserRecord};
use crate::database::tokens::{SessionTokens, TokenStore};
use crate::database::user_roles::UserRoles;
use crate::database::{DatabaseResult, PostgresPool, Table};
use crate::utils::error::DBError;
use crate::utils::{create_salt, hash_password};

/// Table that stores users with their email addresses and hashed passwords
#[derive(Clone)]
pub struct Users {
    pool: PostgresPool,
    user_roles: UserRoles,
    token_store: Arc<Mutex<TokenStore>>,
}

impl Table for Users {
    fn new(pool: PostgresPool) -> Self {
        Self {
            user_roles: UserRoles::new(PostgresPool::clone(&pool)),
            pool,
            token_store: Arc::new(Mutex::new(TokenStore::new())),
        }
    }

    fn init(&self) -> DatabaseResult<()> {
        self.pool.get()?.batch_execute(
            "CREATE TABLE IF NOT EXISTS users (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR(255) NOT NULL,
            email           VARCHAR(255) UNIQUE NOT NULL,
            password_hash   BYTEA NOT NULL,
            salt            BYTEA NOT NULL
        );",
        )?;

        Ok(())
    }
}

impl Users {
    /// Creates a new user and returns an error if the user already exists.
    /// When creating the user first a salt is generated, then the password is hashed
    /// with BCrypt and the given salt. The salt and the hashed password are then stored into the database
    pub fn create_user(
        &self,
        name: String,
        email: String,
        password: String,
    ) -> DatabaseResult<UserRecord> {
        let mut connection = self.pool.get()?;
        let mut password = Zeroizing::new(password);
        log::trace!("Creating user {} with email  {}", name, email);

        if !connection
            .query("SELECT email FROM users WHERE email = $1", &[&email])?
            .is_empty()
        {
            log::trace!("Failed to create user: Record exists!");
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

    pub fn update_user(
        &self,
        old_email: &String,
        name: &String,
        email: &String,
        password: &Option<String>,
    ) -> DatabaseResult<UserInformation> {
        log::trace!(
            "Updating user {} with new entries name: {},  email: {}",
            old_email,
            name,
            email
        );
        let mut connection = self.pool.get()?;
        if connection
            .query_opt("SELECT email FROM users WHERE email = $1", &[&old_email])?
            .is_none()
        {
            log::trace!("Failed to create user: Record doesn't exist!");
            return Err(DBError::RecordDoesNotExist);
        }
        if old_email != email
            && connection
                .query_opt("SELECT email FROM users WHERE email = $1", &[&email])?
                .is_some()
        {
            log::trace!("Failed to create user: New Record exists!");
            return Err(DBError::GenericError(format!(
                "A user for the email {} already exists!",
                email
            )));
        }
        let new_record = if let Some(password) = password {
            let salt = Zeroizing::new(create_salt());
            let pw_hash =
                hash_password(password.as_bytes(), &*salt).map_err(|e| DBError::GenericError(e))?;
            connection.query_one(
                "UPDATE users SET name = $1, email = $2, password_hash = $3, salt = $4 WHERE email = $5 RETURNING *",
                &[&name, &email, &pw_hash.to_vec(), &salt.to_vec(), &old_email],
            )?
        } else {
            connection.query_one(
                "UPDATE users SET name = $1, email = $2 WHERE email = $3 RETURNING *",
                &[&name, &email, &old_email],
            )?
        };

        Ok(serde_postgres::from_row::<UserInformation>(&new_record)?)
    }

    /// Returns information about a user by Id
    pub fn get_user(&self, id: i32) -> DatabaseResult<UserInformation> {
        log::trace!("Looking up entry for user with id {}", id);
        let mut connection = self.pool.get()?;
        let result = connection
            .query_opt("SELECT id, name, email FROM users WHERE id = $1", &[&id])?
            .ok_or(DBError::RecordDoesNotExist)?;

        Ok(serde_postgres::from_row::<UserInformation>(&result)?)
    }

    pub fn get_user_by_email(&self, email: &String) -> DatabaseResult<UserInformation> {
        log::trace!("Looking up entry for user with email {}", email);
        let mut connection = self.pool.get()?;
        let result = connection
            .query_opt(
                "SELECT id, name, email FROM users WHERE email = $1",
                &[email],
            )?
            .ok_or(DBError::RecordDoesNotExist)?;

        Ok(serde_postgres::from_row::<UserInformation>(&result)?)
    }

    pub fn get_users(&self) -> DatabaseResult<Vec<UserInformation>> {
        log::trace!("Returning a list of all users...");
        let mut connection = self.pool.get()?;
        let results = connection.query("SELECT id, name, email FROM users", &[])?;
        let mut users = Vec::new();

        for result in results {
            users.push(serde_postgres::from_row::<UserInformation>(&result)?);
        }

        Ok(users)
    }

    pub fn delete_user(&self, email: &String) -> DatabaseResult<()> {
        log::trace!("Deleting user with email {}", email);
        let mut connection = self.pool.get()?;
        let exists = connection.query_opt("SELECT id FROM users WHERE email = $1", &[email])?;
        if exists.is_none() {
            return Err(DBError::RecordDoesNotExist);
        }
        connection.query("DELETE FROM users WHERE email = $1", &[email])?;

        Ok(())
    }

    /// Creates new tokens for a user login that can be used by services
    /// that need those tokens to verify a user login
    pub fn create_tokens(
        &self,
        email: &String,
        password: &String,
    ) -> DatabaseResult<SessionTokens> {
        log::trace!("Creating new tokens for user with email {}", email);
        if self.validate_login(&email, password)? {
            let mut connection = self.pool.get()?;
            let row = connection.query_one("SELECT id FROM users WHERE email = $1", &[&email])?;
            let id: i32 = row.get(0);

            let tokens = SessionTokens::new(id);
            tokens.store(&mut self.token_store.lock())?;

            Ok(tokens)
        } else {
            Err(DBError::GenericError("Invalid password".to_string()))
        }
    }

    /// Validates a request token and returns if it's valid and the
    /// ttl of the token
    pub fn validate_request_token(&self, token: &String) -> DatabaseResult<(bool, i32)> {
        let mut store = self.token_store.lock();
        let entry = store.get_by_request_token(&token);

        if let Some(entry) = entry {
            Ok((true, entry.request_ttl()))
        } else {
            Ok((false, -1))
        }
    }

    /// Validates a refresh token and returns if it's valid and the ttl
    pub fn validate_refresh_token(&self, token: &String) -> DatabaseResult<(bool, i32)> {
        let mut store = self.token_store.lock();
        let entry = store.get_by_refresh_token(&token);

        if let Some(entry) = entry {
            Ok((true, entry.refresh_ttl()))
        } else {
            Ok((false, -1))
        }
    }

    /// Returns a new request token for a given refresh token
    /// if the refresh token is valid
    pub fn refresh_tokens(&self, refresh_token: &String) -> DatabaseResult<SessionTokens> {
        let mut token_store = self.token_store.lock();
        let tokens = token_store.get_by_refresh_token(refresh_token);

        if let Some(mut tokens) = tokens.and_then(|t| SessionTokens::from_entry(t)) {
            log::trace!("Tokens found. Refreshing...");
            tokens.refresh();
            tokens.store(&mut token_store)?;
            log::trace!("Tokens successfully refreshed.");

            Ok(tokens)
        } else {
            log::trace!("No token store found for user");
            Err(DBError::GenericError("Invalid refresh token!".to_string()))
        }
    }

    pub fn delete_tokens(&self, request_token: &String) -> DatabaseResult<bool> {
        let mut token_store = self.token_store.lock();
        let tokens = token_store.get_by_request_token(request_token);

        if let Some(tokens) = tokens {
            tokens.invalidate();

            Ok(true)
        } else {
            Err(DBError::GenericError("Invalid request token!".to_string()))
        }
    }

    /// Returns if the user has the given permission
    pub fn has_permission(&self, id: i32, permission: &str) -> DatabaseResult<bool> {
        let mut connection = self.pool.get()?;
        let row = connection.query_opt(
            "\
            SELECT * FROM user_roles, role_permissions, permissions
            WHERE user_roles.user_id = $1 
            AND user_roles.role_id = role_permissions.role_id
            AND role_permissions.permission_id = permissions.id
            AND permissions.name = $2
            LIMIT 1
        ",
            &[&id, &permission],
        )?;
        Ok(row.is_some())
    }

    /// Validates the login data of the user by creating the hash for the given password
    /// and comparing it with the database entry
    pub fn validate_login(&self, email: &String, password: &String) -> DatabaseResult<bool> {
        let mut connection = self.pool.get()?;
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

    pub fn get_permissions(&self, email: &String) -> DatabaseResult<Vec<Permission>> {
        let mut connection = self.pool.get()?;
        let results = connection.query(
            "\
            SELECT permissions.id, permissions.name, permissions.description
            FROM permissions, role_permissions, user_roles, users
            WHERE users.email = $1
            AND users.id = user_roles.user_id
            AND role_permissions.role_id = user_roles.role_id
            AND permissions.id = role_permissions.permission_id
        ",
            &[&email],
        )?;
        let permissions: Vec<Permission> = serde_postgres::from_rows(results.iter())?;

        Ok(permissions)
    }
}
