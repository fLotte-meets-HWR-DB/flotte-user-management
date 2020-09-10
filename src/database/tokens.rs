use crate::database::redis_operations::{EX, GET, SET};
use crate::database::RedisConnection;
use crate::utils::create_user_token;
use byteorder::{BigEndian, ByteOrder};
use redis::RedisResult;
use zeroize::Zeroize;

const REQUEST_TOKEN_EXPIRE_SECONDS: usize = 60 * 10;
const REFRESH_TOKEN_EXPIRE_SECONDS: usize = 60 * 60 * 24;

#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct SessionTokens {
    pub request_token: [u8; 32],
    pub refresh_token: [u8; 32],
}

impl SessionTokens {
    pub fn new(user_id: i32) -> Self {
        Self {
            request_token: create_user_token(user_id),
            refresh_token: create_user_token(user_id),
        }
    }

    pub fn from_tokens(request_token: [u8; 32], refresh_token: [u8; 32]) -> Self {
        Self {
            request_token,
            refresh_token,
        }
    }

    pub fn retrieve(user_id: i32, redis_connection: &mut RedisConnection) -> RedisResult<Self> {
        let redis_request_key = format!("user-{}_request", user_id);
        let request_token_vec: Vec<u8> = redis::cmd(GET)
            .arg(redis_request_key)
            .query(redis_connection)?;
        let redis_refresh_key = format!("user-{}_refresh", user_id);
        let refresh_token_vec: Vec<u8> = redis::cmd(GET)
            .arg(redis_refresh_key)
            .query(redis_connection)?;

        let mut request_token = [0u8; 32];
        let mut refresh_token = [0u8; 32];
        if request_token_vec.len() == 32 {
            request_token.copy_from_slice(&request_token_vec);
        }
        if refresh_token_vec.len() == 32 {
            refresh_token.copy_from_slice(&refresh_token_vec);
        }

        Ok(Self {
            request_token,
            refresh_token,
        })
    }

    pub fn refresh(&mut self) {
        self.request_token = create_user_token(self.get_user_id());
        self.refresh_token = create_user_token(self.get_user_id());
    }

    /// Returns the user id that is stored in the first four bytes of the refresh token
    pub fn get_user_id(&self) -> i32 {
        BigEndian::read_i32(&self.refresh_token[0..4])
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
