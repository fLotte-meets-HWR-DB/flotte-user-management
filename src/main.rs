use colored::Colorize;
use crossbeam_utils::sync::WaitGroup;
use env_logger::Env;
use flotte_user_management::database::Database;
use flotte_user_management::server::user_rpc::UserRpcServer;
use log::Level;
use std::thread;

fn main() {
    init_logger();
    let database = Database::new().unwrap();
    database.init().unwrap();
    let rpc_server = UserRpcServer::new(&database);
    let wg = WaitGroup::new();
    {
        let wg = WaitGroup::clone(&wg);
        thread::spawn(move || {
            rpc_server.start();
            std::mem::drop(wg);
        });
    }
    wg.wait();
}

fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            let color = get_level_style(record.level());
            writeln!(
                buf,
                "{}: {}",
                record
                    .level()
                    .to_string()
                    .to_lowercase()
                    .as_str()
                    .color(color),
                record.args()
            )
        })
        .init();
}

fn get_level_style(level: Level) -> colored::Color {
    match level {
        Level::Trace => colored::Color::Magenta,
        Level::Debug => colored::Color::Blue,
        Level::Info => colored::Color::Green,
        Level::Warn => colored::Color::Yellow,
        Level::Error => colored::Color::Red,
    }
}
