use crate::utils::{create_user_token, get_user_id_from_token, TOKEN_LENGTH};
use serde::Serialize;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::Instant;
use zeroize::Zeroize;

const REQUEST_TOKEN_EXPIRE_SECONDS: u32 = 60 * 10;
const REFRESH_TOKEN_EXPIRE_SECONDS: u32 = 60 * 60 * 24;

#[derive(Clone, Debug, Zeroize, Serialize)]
#[zeroize(drop)]
pub struct SessionTokens {
    pub request_token: String,
    pub refresh_token: String,
    pub request_ttl: i32,
    pub refresh_ttl: i32,
}

impl SessionTokens {
    pub fn new(user_id: i32) -> Self {
        Self {
            request_token: base64::encode(create_user_token(user_id)),
            refresh_token: base64::encode(create_user_token(user_id)),
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS as i32,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS as i32,
        }
    }

    pub fn from_tokens(request_token: String, refresh_token: String) -> Self {
        Self {
            request_token,
            refresh_token,
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS as i32,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS as i32,
        }
    }

    pub fn from_entry(other: &TokenStoreEntry) -> Option<Self> {
        let request_token = other.request_token()?;
        let refresh_token = other.refresh_token()?;
        Some(Self {
            refresh_token,
            request_token,
            request_ttl: other.request_ttl(),
            refresh_ttl: other.refresh_ttl(),
        })
    }

    pub fn refresh(&mut self) {
        self.request_token = base64::encode(create_user_token(self.get_user_id()));
    }

    /// Returns the user id that is stored in the first four bytes of the refresh token
    pub fn get_user_id(&self) -> i32 {
        get_user_id_from_token(&self.refresh_token)
    }

    /// Saves the tokens into the database
    pub fn store(&self, token_store: &mut TokenStore) -> Result<(), String> {
        token_store.insert(&self.request_token, &self.refresh_token)
    }
}

#[derive(Clone, Debug)]
pub struct TokenStoreEntry {
    request_token: [u8; TOKEN_LENGTH],
    request_ttl: u32,
    refresh_token: [u8; TOKEN_LENGTH],
    refresh_ttl: u32,
    ttl_start: Instant,
}

impl TokenStoreEntry {
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

    pub fn request_ttl(&self) -> i32 {
        max(
            (self.request_ttl - self.ttl_start.elapsed().as_secs() as u32) as i32,
            -1,
        )
    }

    pub fn refresh_ttl(&self) -> i32 {
        max(
            (self.refresh_ttl - self.ttl_start.elapsed().as_secs() as u32) as i32,
            -1,
        )
    }

    pub fn request_token(&self) -> Option<String> {
        if self.request_ttl() > 0 {
            Some(base64::encode(&self.request_token))
        } else {
            None
        }
    }

    pub fn refresh_token(&self) -> Option<String> {
        if self.refresh_ttl() > 0 {
            Some(base64::encode(&self.refresh_token))
        } else {
            None
        }
    }

    pub fn set_request_token(&mut self, token: String) -> i32 {
        self.request_token
            .copy_from_slice(base64::decode(token).unwrap().as_slice());
        self.reset_timer();
        self.request_ttl = REQUEST_TOKEN_EXPIRE_SECONDS;
        self.refresh_ttl = REFRESH_TOKEN_EXPIRE_SECONDS;

        self.request_ttl as i32
    }

    fn reset_timer(&mut self) {
        self.request_ttl = min(self.request_ttl(), 0) as u32;
        self.refresh_ttl = min(self.refresh_ttl(), 0) as u32;
        self.ttl_start = Instant::now();
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

    pub fn get_request_token(&self, request_token: &String) -> Option<&TokenStoreEntry> {
        let user_id = get_user_id_from_token(&request_token);
        if let Some(user_tokens) = self.tokens.get(&user_id) {
            user_tokens.iter().find(|e| {
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
    pub fn get_refresh_token(&self, refresh_token: &String) -> Option<&TokenStoreEntry> {
        let user_id = get_user_id_from_token(&refresh_token);
        if let Some(user_tokens) = self.tokens.get(&user_id) {
            user_tokens.iter().find(|e| {
                if let Some(token) = e.refresh_token() {
                    &token == refresh_token
                } else {
                    false
                }
            })
        } else {
            None
        }
    }
    pub fn set_request_token(&mut self, refresh_token: &String, request_token: &String) {
        self.clear_expired();
        let user_id = get_user_id_from_token(&request_token);
        if let Some(user_tokens) = self.tokens.get_mut(&user_id) {
            user_tokens.iter_mut().for_each(|e| {
                if let Some(ref_token) = &e.refresh_token() {
                    if ref_token == refresh_token {
                        e.set_request_token(request_token.to_string());
                    }
                }
            });
        }
    }

    pub fn insert(&mut self, request_token: &String, refresh_token: &String) -> Result<(), String> {
        let user_id = get_user_id_from_token(refresh_token);
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

    pub fn clear_expired(&mut self) {
        for (key, entry) in &self.tokens.clone() {
            self.tokens.insert(
                *key,
                entry
                    .iter()
                    .cloned()
                    .filter(|e| e.refresh_ttl() > 0)
                    .collect(),
            );
        }
    }
}
