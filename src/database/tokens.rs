//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use std::cmp::max;
use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;
use zeroize::Zeroize;

use crate::utils::{create_user_token, get_user_id_from_token, TOKEN_LENGTH};

const REQUEST_TOKEN_EXPIRE_SECONDS: u32 = 60 * 10;
const REFRESH_TOKEN_EXPIRE_SECONDS: u32 = 60 * 60 * 24;

/// A struct to store session tokens of a user in a API-readable format
#[derive(Clone, Debug, Zeroize, Serialize, JsonSchema)]
#[zeroize(drop)]
pub struct SessionTokens {
    pub request_token: String,
    pub refresh_token: String,
    pub request_ttl: i32,
    pub refresh_ttl: i32,
}

impl SessionTokens {
    /// Creates a new sessions token entry with newly generated tokens
    pub fn new(user_id: i32) -> Self {
        Self {
            request_token: base64::encode(create_user_token(user_id)),
            refresh_token: base64::encode(create_user_token(user_id)),
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS as i32,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS as i32,
        }
    }

    /// Creates a sessions token entry with the given tokens
    pub fn with_tokens(request_token: String, refresh_token: String) -> Self {
        Self {
            request_token,
            refresh_token,
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS as i32,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS as i32,
        }
    }

    /// Creates a new session tokens instance from a token store
    /// entry
    pub fn from_entry(other: &TokenStoreEntry) -> Option<Self> {
        let request_token = other.request_token().unwrap_or("".to_string());
        let refresh_token = other.refresh_token()?;

        Some(Self {
            refresh_token,
            request_token,
            request_ttl: other.request_ttl(),
            refresh_ttl: other.refresh_ttl(),
        })
    }

    /// Refreshes the request token
    pub fn refresh(&mut self) {
        self.request_token = base64::encode(create_user_token(self.get_user_id()));
        self.request_ttl = REQUEST_TOKEN_EXPIRE_SECONDS as i32;
        log::trace!("Request token refreshed.")
    }

    /// Returns the user id that is stored in the first four bytes of the refresh token
    pub fn get_user_id(&self) -> i32 {
        get_user_id_from_token(&self.refresh_token).unwrap()
    }

    /// Saves the tokens into the database
    pub fn store(&self, token_store: &mut TokenStore) -> Result<(), String> {
        if let Some(tokens) = token_store.get_by_refresh_token(&self.refresh_token) {
            tokens.set_request_token(self.request_token.clone());
        } else {
            token_store.insert(&self.request_token, &self.refresh_token)?;
        }

        Ok(())
    }
}

/// A store entry for tokens that keeps track of the token
/// expirations and provides an abstracted access to those.
/// The tokens are stored as their actual bytes representation
/// to decrease the memory impact
#[derive(Clone, Debug)]
pub struct TokenStoreEntry {
    request_token: [u8; TOKEN_LENGTH],
    request_ttl: u32,
    refresh_token: [u8; TOKEN_LENGTH],
    refresh_ttl: u32,
    ttl_start: Instant,
}

impl TokenStoreEntry {
    /// Creates a new token store entry with the given tokens
    /// and sets the expiration to the configured maximum token lifetime
    pub fn new(request_token: &String, refresh_token: &String) -> Result<Self, String> {
        let request_token = base64::decode(request_token).unwrap();
        let refresh_token = &base64::decode(refresh_token).unwrap();
        if request_token.len() != TOKEN_LENGTH || refresh_token.len() != TOKEN_LENGTH {
            return Err("Invalid token length".to_string());
        }
        let mut req_token = [0u8; TOKEN_LENGTH];
        let mut ref_token = [0u8; TOKEN_LENGTH];
        req_token.copy_from_slice(&request_token);
        ref_token.copy_from_slice(&refresh_token);

        Ok(Self {
            request_token: req_token,
            refresh_token: ref_token,
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS,
            ttl_start: Instant::now(),
        })
    }

    /// Returns the ttl for the request token that is
    /// calculated from the stored instant.
    /// If the token is expired -1 is returned.
    pub fn request_ttl(&self) -> i32 {
        max(
            self.request_ttl as i64 - self.ttl_start.elapsed().as_secs() as i64,
            -1,
        ) as i32
    }

    /// Returns the ttl for the refresh token
    /// that is calculated from the stored instant.
    /// If the token is expired -1 is returned.
    pub fn refresh_ttl(&self) -> i32 {
        max(
            self.refresh_ttl as i64 - self.ttl_start.elapsed().as_secs() as i64,
            -1,
        ) as i32
    }

    /// Returns the request token if it hasn't expired
    pub fn request_token(&self) -> Option<String> {
        if self.request_ttl() > 0 {
            Some(base64::encode(&self.request_token))
        } else {
            None
        }
    }

    /// Returns the refresh token if it hasn't expired
    pub fn refresh_token(&self) -> Option<String> {
        if self.refresh_ttl() > 0 {
            Some(base64::encode(&self.refresh_token))
        } else {
            None
        }
    }

    /// Sets a new request token and resets
    /// the expiration time for the request and refresh token
    pub fn set_request_token(&mut self, token: String) -> i32 {
        self.request_token
            .copy_from_slice(base64::decode(token).unwrap().as_slice());
        self.reset_timer();
        self.request_ttl = REQUEST_TOKEN_EXPIRE_SECONDS;
        self.refresh_ttl = REFRESH_TOKEN_EXPIRE_SECONDS;
        log::trace!("TTLs reset");

        self.request_ttl as i32
    }

    /// Resets the timer that keeps track of the tokens expiration times
    /// before resetting the current expiration times are stored so the ttl
    /// for both tokens won't reset
    fn reset_timer(&mut self) {
        log::trace!("Resetting timer...");
        log::trace!(
            "req_ttl: {}, ref_ttl: {}",
            self.request_ttl,
            self.refresh_ttl
        );
        self.request_ttl = max(self.request_ttl(), 0) as u32;
        self.refresh_ttl = max(self.refresh_ttl(), 0) as u32;
        self.ttl_start = Instant::now();
        log::trace!(
            "req_ttl: {}, ref_ttl: {}",
            self.request_ttl,
            self.refresh_ttl
        );
    }

    /// Invalidates the token entry which causes it to be deleted with the next
    /// clearing of expired tokens by the token store. The
    pub(crate) fn invalidate(&mut self) {
        self.request_ttl = 0;
        self.refresh_ttl = 0;
        log::trace!("Tokens invalidated.");
    }
}

#[derive(Debug)]
pub struct TokenStore {
    tokens: HashMap<i32, Vec<TokenStoreEntry>>,
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Returns the token store entry for a given request token
    pub fn get_by_request_token(&mut self, request_token: &String) -> Option<&mut TokenStoreEntry> {
        let user_id = get_user_id_from_token(&request_token)?;

        if let Some(user_tokens) = self.tokens.get_mut(&user_id) {
            user_tokens.iter_mut().find(|e| {
                if let Some(token) = e.request_token() {
                    &token == request_token
                } else {
                    false
                }
            })
        } else {
            None
        }
    }

    /// Returns the token store entry by the given refresh token
    pub fn get_by_refresh_token(&mut self, refresh_token: &String) -> Option<&mut TokenStoreEntry> {
        log::trace!("Retrieving user by refresh token.");
        let user_id = get_user_id_from_token(&refresh_token)?;
        log::trace!("UserID is {}", user_id);

        if let Some(user_tokens) = self.tokens.get_mut(&user_id) {
            user_tokens.iter_mut().find(|e| {
                if let Some(token) = e.refresh_token() {
                    token.eq(refresh_token)
                } else {
                    false
                }
            })
        } else {
            log::trace!("No tokens found for user");
            None
        }
    }

    /// Sets the request token for a given refresh token
    /// Also clears all expired token entries.
    pub fn set_request_token(&mut self, refresh_token: &String, request_token: &String) {
        self.clear_expired();
        let user_id = get_user_id_from_token(&request_token).unwrap();
        if let Some(user_tokens) = self.tokens.get_mut(&user_id) {
            user_tokens.iter_mut().for_each(|e| {
                if let Some(ref_token) = &e.refresh_token() {
                    if ref_token == refresh_token {
                        e.set_request_token(request_token.to_string());
                        log::debug!("New request token set for userId {}", user_id);
                    }
                }
            });
        }
    }

    /// Inserts a new pair of request and refresh token
    pub fn insert(&mut self, request_token: &String, refresh_token: &String) -> Result<(), String> {
        let user_id =
            get_user_id_from_token(refresh_token).ok_or("Invalid request token".to_string())?;
        let user_tokens = if let Some(user_tokens) = self.tokens.get_mut(&user_id) {
            user_tokens
        } else {
            self.tokens.insert(user_id, Vec::new());

            self.tokens.get_mut(&user_id).unwrap()
        };
        if let Some(tokens) = user_tokens.iter_mut().find(|t| {
            if let Some(token) = t.refresh_token() {
                &token == refresh_token
            } else {
                false
            }
        }) {
            tokens.set_request_token(request_token.clone());
        } else {
            let entry = TokenStoreEntry::new(request_token, refresh_token)?;
            user_tokens.push(entry);
        }

        Ok(())
    }

    /// Deletes all expired tokens from the store
    pub fn clear_expired(&mut self) {
        log::trace!("Clearing expired tokens...");
        for (key, entry) in &self.tokens.clone() {
            log::trace!("Before: {} tokens for user {}", entry.len(), key);
            self.tokens.insert(
                *key,
                entry
                    .iter()
                    .cloned()
                    .filter(|e| e.refresh_ttl() > 0)
                    .collect(),
            );
            log::trace!(
                "After: {} tokens for user {}",
                self.tokens.get(key).unwrap().len(),
                key
            );
        }
        log::trace!("Clearing expired tokens cleared");
    }
}
