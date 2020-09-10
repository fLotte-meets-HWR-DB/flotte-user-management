use rand::Rng;

pub fn create_salt(length: usize) -> [u8; 16] {
    let mut rng = rand::thread_rng();
    let mut salt = [0u8; 16];
    rng.fill(&mut salt);

    salt
}
