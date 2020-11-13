//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use std::error::Error;
use std::fmt::{self, Display};
use std::io::Read;

use regex::Regex;
use rouille::{Request, Response, Server};
use serde::export::Formatter;
use serde::Serialize;

use crate::database::models::Role;
use crate::database::permissions::{
    CREATE_ROLE_PERMISSION, UPDATE_ROLE_PERMISSION, VIEW_ROLE_PERMISSION,
};
use crate::database::tokens::SessionTokens;
use crate::database::Database;
use crate::server::documentation::RESTDocumentation;
use crate::server::messages::{
    ErrorMessage, FullRoleData, LoginMessage, LogoutConfirmation, LogoutMessage, ModifyRoleRequest,
    RefreshMessage,
};
use crate::utils::error::DBError;
use crate::utils::get_user_id_from_token;

macro_rules! require_permission {
    ($database:expr,$request:expr,$permission:expr) => {
        let (_token, id) = validate_request_token($request, $database)?;
        if !$database.users.has_permission(id, $permission)? {
            return Err(HTTPError::new("Insufficient permissions".to_string(), 403));
        }
    };
}

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
                (GET) (/info) => {
                    Self::info(request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/login) => {
                    Self::login(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/new-token) => {
                    Self::new_token(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/logout) => {
                    Self::logout(&database, request).unwrap_or_else(HTTPError::into)
                },
                (GET) (/roles/{name: String}) => {
                    Self::get_role(&database, request, name).unwrap_or_else(HTTPError::into)
                },
                (GET) (/roles) => {
                    Self::get_roles(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/roles/create) => {
                    Self::create_role(&database, request).unwrap_or_else(HTTPError::into)
                },
                (POST) (/roles/update) => {
                    Self::update_role(&database, request).unwrap_or_else(HTTPError::into)
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
                    .with_additional_header("Vary", "Access-Control-Request-Headers");

                if let Some(request_headers) = request.header("Access-Control-Request-Headers") {
                    response = response.with_additional_header(
                        "Access-Control-Allow-Headers",
                        request_headers.to_string(),
                    );
                }
            }

            response
        })
        .unwrap();
        log::info!("HTTP-Server running on {}", listen_address);
        server.run()
    }

    fn build_docs() -> Result<RESTDocumentation, serde_json::Error> {
        let mut doc = RESTDocumentation::new("/info");
        doc.add_path::<LoginMessage, SessionTokens>(
            "/login",
            "POST",
            "Returns request and refresh tokens",
        )?;
        doc.add_path::<RefreshMessage, SessionTokens>(
            "/new-token",
            "POST",
            "Returns a new request token",
        )?;
        doc.add_path::<LogoutMessage, LogoutConfirmation>(
            "/logout",
            "POST",
            "Invalidates the refresh and request tokens",
        )?;
        doc.add_path::<(), FullRoleData>(
            "/roles/{name:String}",
            "GET",
            "Returns the role with the given name",
        )?;
        doc.add_path::<(), Vec<Role>>("/roles", "GET", "Returns a list of all roles")?;
        doc.add_path::<ModifyRoleRequest, FullRoleData>(
            "/roles/create",
            "POST",
            "Creates a new role",
        )?;
        doc.add_path::<ModifyRoleRequest, FullRoleData>(
            "/roles/update",
            "POST",
            "Updates an existing role",
        )?;

        Ok(doc)
    }

    fn info(request: &Request) -> HTTPResult<Response> {
        lazy_static::lazy_static! {static ref DOCS: RESTDocumentation = UserHttpServer::build_docs().unwrap();}

        Ok(Response::html(
            DOCS.get(request.get_param("path").unwrap_or("/".to_string())),
        ))
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

    /// Returns the data for a given role
    fn get_role(database: &Database, request: &Request, name: String) -> HTTPResult<Response> {
        require_permission!(database, request, VIEW_ROLE_PERMISSION);
        let role = database.roles.get_role(name)?;
        let permissions = database.role_permission.by_role(role.id)?;

        Ok(Response::json(&FullRoleData {
            id: role.id,
            name: role.name,
            permissions,
        }))
    }

    /// Returns a list of all roles
    fn get_roles(database: &Database, request: &Request) -> HTTPResult<Response> {
        require_permission!(database, request, VIEW_ROLE_PERMISSION);
        let roles = database.roles.get_roles()?;

        Ok(Response::json(&roles))
    }

    fn create_role(database: &Database, request: &Request) -> HTTPResult<Response> {
        require_permission!(database, request, CREATE_ROLE_PERMISSION);
        let message: ModifyRoleRequest = serde_json::from_str(parse_string_body(request)?.as_str())
            .map_err(|e| HTTPError::new(e.to_string(), 400))?;
        let not_existing = database
            .permissions
            .get_not_existing(&message.permissions)?;
        if !not_existing.is_empty() {
            return Ok(Response::json(&ErrorMessage::new(format!(
                "The permissions {:?} don't exist",
                not_existing
            )))
            .with_status_code(400));
        }
        let role =
            database
                .roles
                .create_role(message.name, message.description, message.permissions)?;
        let permissions = database.role_permission.by_role(role.id)?;

        Ok(Response::json(&FullRoleData {
            id: role.id,
            permissions,
            name: role.name,
        })
        .with_status_code(201))
    }

    fn update_role(database: &Database, request: &Request) -> HTTPResult<Response> {
        require_permission!(database, request, UPDATE_ROLE_PERMISSION);
        let message: ModifyRoleRequest = serde_json::from_str(parse_string_body(request)?.as_str())
            .map_err(|e| HTTPError::new(e.to_string(), 400))?;
        let not_existing = database
            .permissions
            .get_not_existing(&message.permissions)?;
        if !not_existing.is_empty() {
            return Ok(Response::json(&ErrorMessage::new(format!(
                "The permissions {:?} don't exist",
                not_existing
            )))
            .with_status_code(400));
        }
        let role =
            database
                .roles
                .update_role(message.name, message.description, message.permissions)?;
        let permissions = database.role_permission.by_role(role.id)?;

        Ok(Response::json(&FullRoleData {
            id: role.id,
            permissions,
            name: role.name,
        }))
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

/// Parses and validates the request token from the http header
fn validate_request_token(request: &Request, database: &Database) -> HTTPResult<(String, i32)> {
    lazy_static::lazy_static! {static ref BEARER_REGEX: Regex = Regex::new(r"^[bB]earer\s+").unwrap();}
    let token = request
        .header("authorization")
        .ok_or(HTTPError::new("401 Unauthorized".to_string(), 401))?;
    let token = BEARER_REGEX.replace(token, "");
    let (valid, _) = database.users.validate_request_token(&token.to_string())?;
    if !valid {
        Err(HTTPError::new("Invalid request token".to_string(), 401))
    } else {
        Ok((
            token.to_string(),
            get_user_id_from_token(&token.to_string())
                .ok_or(HTTPError::new("Invalid request token".to_string(), 401))?,
        ))
    }
}
