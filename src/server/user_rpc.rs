use super::rpc_methods::*;
use crate::database::Database;
use crate::server::messages::{
    CreatePermissionsRequest, CreateRoleRequest, ErrorMessage, GetPermissionsRequest, InfoEntry,
    TokenRequest,
};
use crate::utils::get_user_id_from_token;
use msgrpc::message::Message;
use msgrpc::server::RpcServer;
use rmp_serde::Deserializer;
use scheduled_thread_pool::ScheduledThreadPool;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::Builder;

const RPC_SERVER_ADDRESS: &str = "RPC_SERVER_ADDRESS";
const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:5555";

/// The RPC server that provides an interface
/// for applications to validate request tokens
/// and request the assigned roles
pub struct UserRpcServer {
    database: Database,
}

type RpcResult<T> = Result<T, ErrorMessage>;

impl UserRpcServer {
    pub fn new(database: &Database) -> Self {
        Self {
            database: Database::clone(database),
        }
    }

    /// Stats the user rpc server with 2 x num-cpus threads.
    pub fn start(&self) {
        let listen_address =
            dotenv::var(RPC_SERVER_ADDRESS).unwrap_or(DEFAULT_SERVER_ADDRESS.to_string());
        log::info!("Starting RPC-Server...");
        let mut server = RpcServer::new(listen_address.clone());
        let receiver = Arc::clone(&server.receiver);
        Builder::new()
            .name("tcp-receiver".to_string())
            .spawn(move || {
                server.start().unwrap();
            })
            .unwrap();
        let pool = ScheduledThreadPool::new(num_cpus::get());
        log::info!("RPC-Server running on {}", listen_address);
        while let Ok(h) = receiver.lock().unwrap().recv() {
            let database = Database::clone(&self.database);
            log::trace!("Scheduling message for execution in pool");
            pool.execute(move || {
                let mut handler = h.lock().unwrap();
                log::debug!("Received message {:?}", handler.message);
                let response = match handler.message.method {
                    INFO => Self::handle_info(),
                    GET_ROLES => Self::handle_get_roles(database, &handler.message.data),
                    VALIDATE_TOKEN => Self::handle_validate_token(database, &handler.message.data),
                    GET_ROLE_PERMISSIONS => {
                        Self::handle_get_permissions(database, &handler.message.data)
                    }
                    CREATE_ROLE => Self::handle_create_role(database, &handler.message.data),
                    CREATE_PERMISSION => {
                        Self::handle_create_permissions(database, &handler.message.data)
                    }
                    _ => Err(ErrorMessage::new("Invalid Method".to_string())),
                }
                .unwrap_or_else(|e| Message::new_with_serialize(ERROR, e));
                log::debug!("Responding with message {:?}", &response);
                handler.done(response);
            });
        }
    }

    /// Handles the validation of request tokens
    fn handle_validate_token(database: Database, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Validating token.");
        let message = TokenRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let valid = database
            .users
            .validate_request_token(&message.token)
            .unwrap_or((false, -1));
        log::trace!("Serializing...");
        let data = rmp_serde::to_vec(&valid).map_err(|e| ErrorMessage::new(e.to_string()))?;

        Ok(Message::new(VALIDATE_TOKEN, data))
    }

    /// Handles a INFO message that returns all valid methods of the rpc sserver
    fn handle_info() -> RpcResult<Message> {
        log::trace!("Get Info");
        Ok(Message::new_with_serialize(
            INFO,
            vec![
                InfoEntry::new("info", INFO, "Shows this entry", ""),
                InfoEntry::new(
                    "validate token",
                    VALIDATE_TOKEN,
                    "Validates a request token",
                    "{token: String}",
                ),
                InfoEntry::new(
                    "get roles",
                    GET_ROLES,
                    "Returns the roles the user is assigned to",
                    "{token: String}",
                ),
                InfoEntry::new(
                    "get permissions",
                    GET_ROLE_PERMISSIONS,
                    "Returns all permissions the given roles are assigned to",
                    "{roles: [i32]}",
                ),
                InfoEntry::new(
                    "create role",
                    CREATE_ROLE,
                    "Creates a new role with the given permissions",
                    "{name: String, description: String, permissions: [i32]}",
                ),
                InfoEntry::new(
                    "create permissions",
                    CREATE_PERMISSION,
                    "Creates all given permissions if they don't exist.",
                    "{permissions: [{name: String, description: String}]}",
                ),
            ],
        ))
    }

    /// Returns all permissions of a role
    fn handle_get_permissions(database: Database, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Get Permissions");
        let message =
            GetPermissionsRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
                .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let mut response_data = HashMap::new();
        for role_id in message.roles {
            let permissions = database.role_permission.by_role(role_id)?;
            response_data.insert(role_id.to_string(), permissions);
        }

        Ok(Message::new_with_serialize(
            GET_ROLE_PERMISSIONS,
            response_data,
        ))
    }

    /// Returns all roles of a user
    fn handle_get_roles(database: Database, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Get Roles");
        let message = TokenRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        if !database
            .users
            .validate_request_token(&message.token)
            .unwrap_or((false, -1))
            .0
        {
            return Err(ErrorMessage::new("Invalid request token".to_string()));
        }
        let user_id = get_user_id_from_token(&message.token)
            .ok_or(ErrorMessage::new("Invalid request token".to_string()))?;
        let response_data = database.user_roles.by_user(user_id)?;

        Ok(Message::new_with_serialize(GET_ROLES, response_data))
    }

    /// Handles the requests for creating new roles
    fn handle_create_role(database: Database, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Create Role");
        let message = CreateRoleRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let role =
            database
                .roles
                .create_role(message.name, message.description, message.permissions)?;

        Ok(Message::new_with_serialize(CREATE_ROLE, role))
    }

    /// Handles the request for creating new permissions.
    fn handle_create_permissions(database: Database, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Create Permission");
        let message =
            CreatePermissionsRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
                .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let permissions = database
            .permissions
            .create_permissions(message.permissions)?;

        Ok(Message::new_with_serialize(CREATE_PERMISSION, permissions))
    }
}
