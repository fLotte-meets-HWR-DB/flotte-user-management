use flotte_user_management::database::{get_connection, init_database};

fn main() {
    let mut client = get_connection().unwrap();
    init_database(&mut client).unwrap()
}
