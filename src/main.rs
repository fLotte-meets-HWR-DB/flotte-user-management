use flotte_user_management::database::Database;

fn main() {
    let database = Database::new().unwrap();
    database.init().unwrap();
}
