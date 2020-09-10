use flotte_user_management::database::Database;

fn main() {
    let database = Database::new().unwrap();
    database.init().unwrap();
    println!(
        "{:?}",
        database
            .users
            .create_user(
                "John Doe".to_string(),
                "johndoe@protonmail.com".to_string(),
                "ttest".to_string()
            )
            .unwrap()
    )
}
