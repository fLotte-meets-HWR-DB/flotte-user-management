use postgres::Row;
use zeroize::Zeroize;

#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct UserRecord {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Vec<u8>,
    pub salt: Vec<u8>,
}

impl UserRecord {
    pub fn from_ordered_row(row: &Row) -> Self {
        Self {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            password_hash: row.get(3),
            salt: row.get(4),
        }
    }
}
