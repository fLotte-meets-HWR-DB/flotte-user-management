//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use std::panic;

use bcrypt::DEFAULT_COST;
use byteorder::{BigEndian, ByteOrder};
use rand::Rng;

pub mod error;

pub const TOKEN_LENGTH: usize = 32;
const SALT_LENGTH: usize = 16;

/// Creates a new random salt
pub fn create_salt() -> [u8; SALT_LENGTH] {
    let mut rng = rand::thread_rng();
    let mut salt = [0u8; SALT_LENGTH];
    rng.fill(&mut salt);

    salt
}

/// Creates a new random user token where the first 4 bytes represent
/// the userId
pub fn create_user_token(user_id: i32) -> [u8; TOKEN_LENGTH] {
    let mut rng = rand::thread_rng();
    let mut value = [0u8; TOKEN_LENGTH];
    rng.fill(&mut value);
    BigEndian::write_i32(&mut value, user_id);

    value
}

/// Extracts the userId from a request token
pub fn get_user_id_from_token(token: &String) -> Option<i32> {
    let token = base64::decode(&token).ok()?;
    if token.len() > 4 {
        Some(BigEndian::read_i32(token.as_slice()))
    } else {
        None
    }
}

/// Hashes a password with a salt by using BCrypt
pub fn hash_password(password: &[u8], salt: &[u8]) -> Result<[u8; 24], String> {
    panic::catch_unwind(|| {
        let mut pw_hash = [0u8; 24];
        bcrypt::bcrypt(DEFAULT_COST, salt, password, &mut pw_hash);
        Ok(pw_hash)
    })
    .map_err(|_| "Hashing failed".to_string())?
}
