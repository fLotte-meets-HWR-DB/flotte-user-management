use byteorder::{BigEndian, ByteOrder};
use rand::Rng;

pub const TOKEN_LENGTH: usize = 32;
const SALT_LENGTH: usize = 16;

pub fn create_salt() -> [u8; SALT_LENGTH] {
    let mut rng = rand::thread_rng();
    let mut salt = [0u8; SALT_LENGTH];
    rng.fill(&mut salt);

    salt
}

pub fn create_user_token(user_id: i32) -> [u8; TOKEN_LENGTH] {
    let mut rng = rand::thread_rng();
    let mut value = [0u8; TOKEN_LENGTH];
    rng.fill(&mut value);
    BigEndian::write_i32(&mut value, user_id);

    value
}

pub fn get_user_id_from_token(token: &[u8]) -> i32 {
    BigEndian::read_i32(token)
}
