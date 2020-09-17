use crate::database::Database;
use crate::server::messages::{LoginMessage, LogoutConfirmation, LogoutMessage, RefreshMessage};
use crate::utils::error::DBError;
use rouille::{Request, Response, Server};
use serde::export::Formatter;
use serde::Serialize;
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Read;

const LISTEN_ADDRESS: &str = "HTTP_SERVER_ADDRESS";
const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:8080";
const ENV_ENABLE_CORS: &str = "ENABLE_CORS";

/// The HTTP server of the user management that provides a
/// REST api for login and requesting tokens
pub struct UserHttpServer {
    database: Database,
}

#[derive(Debug, Serialize)]
pub struct HTTPError {
    message: String,
    error_code: u16,
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
            error_code: 400,
        }
    }
}

impl Into<Response> for HTTPError {
    fn into(self) -> Response {
        Response::json(&self).with_status_code(self.error_code)
    }
}

impl HTTPError {
    pub fn new(message: String, code: u16) -> Self {
        Self {
            message,
            error_code: code,
        }
    }
}

type HTTPResult<T> = Result<T, HTTPError>;

impl UserHttpServer {
    pub fn new(database: &Database) -> Self {
        Self {
            database: Database::clone(database),
        }
    }

    /// Stats the server.
    /// This call blocks until the server is shut down.
    pub fn start(&self) {
        log::info!("Starting HTTP-Server...");
        let listen_address =
            dotenv::var(LISTEN_ADDRESS).unwrap_or(DEFAULT_LISTEN_ADDRESS.to_string());
        let database = Database::clone(&self.database);
        let server = Server::new(&listen_address, move |request| {
            let mut response = router!(request,
                (POST) (/login) => {
                    Self::login(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/new-token) => {
                    Self::new_token(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/logout) => {
                    Self::logout(&database, request).unwrap_or_else(HTTPError::into)
                },
                _ => if request.method() == "OPTIONS" {
                    Response::empty_204()
                } else {
                    Response::empty_404()
                }
            );

            if dotenv::var(ENV_ENABLE_CORS).unwrap_or("false".to_string()) == "true" {
                response = response
                    .with_additional_header("Access-Control-Allow-Origin", "*")
                    .with_additional_header(
                        "Access-Control-Allow-Methods",
                        "GET,HEAD,PUT,PATCH,POST,DELETE",
                    )
                    .with_additional_header("Vary", "Access-Control-Request-Headers")
                    .with_additional_header(
                        "Access-Control-Request-Headers",
                        request
                            .header("Access-Control-Request-Headers")
                            .unwrap_or("content-type")
                            .to_string(),
                    );
            }

            response
        })
        .unwrap();
        log::info!("HTTP-Server running on {}", listen_address);
        server.run()
    }

    /// Handles the login part of the REST api
    fn login(database: &Database, request: &Request) -> HTTPResult<Response> {
        let login_request: LoginMessage =
            serde_json::from_str(parse_string_body(request)?.as_str())
                .map_err(|e| HTTPError::new(e.to_string(), 400))?;

        let tokens = database
            .users
            .create_tokens(&login_request.email, &login_request.password)?;

        Ok(Response::json(&tokens).with_status_code(201))
    }

    /// Handles the new token part of the rest api
    fn new_token(database: &Database, request: &Request) -> HTTPResult<Response> {
        let message: RefreshMessage = serde_json::from_str(parse_string_body(request)?.as_str())
            .map_err(|e| HTTPError::new(e.to_string(), 400))?;

        let tokens = database.users.refresh_tokens(&message.refresh_token)?;

        Ok(Response::json(&tokens))
    }

    fn logout(database: &Database, request: &Request) -> HTTPResult<Response> {
        let message: LogoutMessage = serde_json::from_str(parse_string_body(request)?.as_str())
            .map_err(|e| HTTPError::new(e.to_string(), 400))?;
        let success = database.users.delete_tokens(&message.request_token)?;

        Ok(Response::json(&LogoutConfirmation { success }).with_status_code(205))
    }
}

/// Parses the body of a http request into a string representation
fn parse_string_body(request: &Request) -> HTTPResult<String> {
    let mut body = request
        .data()
        .ok_or(HTTPError::new("Missing request data!".to_string(), 400))?;
    let mut string_body = String::new();
    body.read_to_string(&mut string_body)
        .map_err(|e| HTTPError::new(format!("Failed to parse request data {}", e), 400))?;

    Ok(string_body)
}
