use crate::database::Database;
use crate::server::messages::{LoginMessage, RefreshMessage};
use crate::utils::error::DBError;
use rouille::{Request, Response, Server};
use serde::export::Formatter;
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Read;

const LISTEN_ADDRESS: &str = "HTTP_SERVER_ADDRESS";
const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:8080";

pub struct UserHttpServer {
    database: Database,
}

#[derive(Debug)]
pub struct HTTPError {
    message: String,
    code: u16,
}

impl Display for HTTPError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl Error for HTTPError {}

impl From<DBError> for HTTPError {
    fn from(other: DBError) -> Self {
        Self {
            message: other.to_string(),
            code: 400,
        }
    }
}

impl Into<Response> for HTTPError {
    fn into(self) -> Response {
        Response::text(self.message).with_status_code(self.code)
    }
}

impl HTTPError {
    pub fn new(message: String, code: u16) -> Self {
        Self { message, code }
    }
}

type HTTPResult<T> = Result<T, HTTPError>;

impl UserHttpServer {
    pub fn new(database: &Database) -> Self {
        Self {
            database: Database::clone(database),
        }
    }

    pub fn start(&self) {
        let listen_address =
            dotenv::var(LISTEN_ADDRESS).unwrap_or(DEFAULT_LISTEN_ADDRESS.to_string());
        let database = Database::clone(&self.database);
        let server = Server::new(listen_address, move |request| {
            router!(request,
                (POST) (/login) => {
                    Self::login(&database, request).unwrap_or_else(|e|e.into())
                },
                (POST) (/new-token) => {
                    Self::new_token(&database, request).unwrap_or_else(|e|e.into())
                },
                _ => Response::empty_404()
            )
        })
        .unwrap();
        server.run()
    }

    fn login(database: &Database, request: &Request) -> HTTPResult<Response> {
        if let Some(mut data) = request.data() {
            let mut data_string = String::new();
            data.read_to_string(&mut data_string)
                .map_err(|_| HTTPError::new("Failed to read request data".to_string(), 500))?;
            let login_request: LoginMessage = serde_json::from_str(data_string.as_str())
                .map_err(|e| HTTPError::new(e.to_string(), 400))?;
            let tokens = database
                .users
                .create_tokens(&login_request.email, &login_request.password)?;

            Ok(Response::json(&tokens))
        } else {
            Err(HTTPError::new("Missing Request Data".to_string(), 400))
        }
    }

    fn new_token(database: &Database, request: &Request) -> HTTPResult<Response> {
        if let Some(mut data) = request.data() {
            let mut data_string = String::new();
            data.read_to_string(&mut data_string)
                .map_err(|_| HTTPError::new("Failed to read request data".to_string(), 500))?;
            let message: RefreshMessage = serde_json::from_str(data_string.as_str())
                .map_err(|e| HTTPError::new(e.to_string(), 400))?;

            let tokens = database.users.refresh_tokens(&message.refresh_token)?;

            Ok(Response::json(&tokens))
        } else {
            Err(HTTPError::new("Missing Request Data".to_string(), 400))
        }
    }
}
