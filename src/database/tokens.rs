use crate::database::redis_operations::{EX, GET, SET, TTL};
use crate::utils::error::RedisConnection;
use crate::utils::{create_user_token, get_user_id_from_token};
use byteorder::{BigEndian, ByteOrder};
use redis::{ErrorKind, RedisError, RedisResult};
use serde::Serialize;
use zeroize::Zeroize;

const REQUEST_TOKEN_EXPIRE_SECONDS: i32 = 60 * 10;
const REFRESH_TOKEN_EXPIRE_SECONDS: i32 = 60 * 60 * 24;

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
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS,
        }
    }

    pub fn from_tokens(request_token: String, refresh_token: String) -> Self {
        Self {
            request_token,
            refresh_token,
            request_ttl: REQUEST_TOKEN_EXPIRE_SECONDS,
            refresh_ttl: REFRESH_TOKEN_EXPIRE_SECONDS,
        }
    }

    pub fn retrieve(user_id: i32, redis_connection: &mut RedisConnection) -> RedisResult<Self> {
        let redis_request_key = format!("user-{}_request", user_id);
        let request_token: String = redis::cmd(GET)
            .arg(&redis_request_key)
            .query(redis_connection)?;
        let redis_refresh_key = format!("user-{}_refresh", user_id);
        let refresh_token: String = redis::cmd(GET)
            .arg(&redis_refresh_key)
            .query(redis_connection)?;

        if request_token.len() == 0 {
            return Err(RedisError::from((
                ErrorKind::ResponseError,
                "No refresh token available",
            )));
        }
        if refresh_token.len() == 0 {
            return Err(RedisError::from((
                ErrorKind::ResponseError,
                "No refresh token available",
            )));
        }
        let request_ttl: i32 = redis::cmd(TTL)
            .arg(&redis_request_key)
            .query(redis_connection)?;
        let refresh_ttl: i32 = redis::cmd(TTL)
            .arg(&redis_refresh_key)
            .query(redis_connection)?;

        Ok(Self {
            request_token,
            refresh_token,
            request_ttl,
            refresh_ttl,
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
    pub fn store(&self, redis_connection: &mut RedisConnection) -> RedisResult<()> {
        let id = self.get_user_id();

        let redis_request_key = format!("user-{}_request", id);
        redis::cmd(SET)
            .arg(&redis_request_key)
            .arg(&self.request_token)
            .arg(EX)
            .arg(REQUEST_TOKEN_EXPIRE_SECONDS)
            .query(&mut *redis_connection)?;

        let redis_refresh_key = format!("user-{}_refresh", id);

        redis::cmd(SET)
            .arg(&redis_refresh_key)
            .arg(&self.refresh_token)
            .arg(EX)
            .arg(REFRESH_TOKEN_EXPIRE_SECONDS)
            .query(&mut *redis_connection)?;

        Ok(())
    }
}
