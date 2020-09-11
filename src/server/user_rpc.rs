use super::rpc_methods::*;
use crate::database::Database;
use crate::server::messages::{
    CreateRoleRequest, ErrorMessage, GetPermissionsRequest, InfoEntry, TokenRequest,
};
use crate::utils::get_user_id_from_token;
use msgrpc::message::Message;
use msgrpc::server::RpcServer;
use rmp_serde::Deserializer;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

const RPC_SERVER_ADDRESS: &str = "RPC_SERVER_ADDRESS";
const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:5555";

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

    pub fn start(&self) {
        let mut server = RpcServer::new(
            dotenv::var(RPC_SERVER_ADDRESS).unwrap_or(DEFAULT_SERVER_ADDRESS.to_string()),
        );
        let receiver = Arc::clone(&server.receiver);
        thread::spawn(move || {
            server.start().unwrap();
        });
        while let Ok(h) = receiver.lock().unwrap().recv() {
            let mut handler = h.lock().unwrap();
            let response = match handler.message.method {
                INFO => self.handle_info(),
                GET_ROLES => self.handle_get_roles(&handler.message.data),
                VALIDATE_TOKEN => self.handle_validate_token(&handler.message.data),
                GET_ROLE_PERMISSIONS => self.handle_get_permissions(&handler.message.data),
                CREATE_ROLE => self.handle_create_role(&handler.message.data),
                _ => Err(ErrorMessage::new("Invalid Method".to_string())),
            }
            .unwrap_or_else(|e| Message::new_with_serialize(ERROR, e));
            log::trace!("Responding with {:?}", &response);
            handler.done(response);
        }
    }

    fn handle_validate_token(&self, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Validating token.");
        let message = TokenRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let valid = self
            .database
            .users
            .validate_request_token(&message.token)
            .unwrap_or((false, -1));
        log::trace!("Serializing...");
        let data = rmp_serde::to_vec(&valid).map_err(|e| ErrorMessage::new(e.to_string()))?;

        Ok(Message::new(VALIDATE_TOKEN, data))
    }

    fn handle_info(&self) -> RpcResult<Message> {
        log::trace!("Get Info");
        Ok(Message::new_with_serialize(
            INFO,
            vec![
                InfoEntry::new("info", INFO, "Shows this entry", ""),
                InfoEntry::new(
                    "validate token",
                    VALIDATE_TOKEN,
                    "Validates a request token",
                    "{token: [u8; 32]}",
                ),
                InfoEntry::new(
                    "get roles",
                    GET_ROLES,
                    "Returns the roles the user is assigned to",
                    "{token: [u8; 32]}",
                ),
                InfoEntry::new(
                    "get permissions",
                    GET_ROLE_PERMISSIONS,
                    "Returns all permissions the given roles are assigned to",
                    "{role_ids: [i32]}",
                ),
                InfoEntry::new(
                    "create role",
                    CREATE_ROLE,
                    "Creates a new role with the given permissions",
                    "{name: String, description: String, permissions: [i32]}",
                ),
            ],
        ))
    }

    fn handle_get_permissions(&self, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Get Permissions");
        let message =
            GetPermissionsRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
                .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let mut response_data = HashMap::new();
        for role_id in message.role_ids {
            let permissions = self.database.role_permission.by_role(role_id)?;
            response_data.insert(role_id.to_string(), permissions);
        }

        Ok(Message::new_with_serialize(
            GET_ROLE_PERMISSIONS,
            response_data,
        ))
    }

    fn handle_get_roles(&self, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Get Roles");
        let message = TokenRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        if !self
            .database
            .users
            .validate_request_token(&message.token)
            .unwrap_or((false, -1))
            .0
        {
            return Err(ErrorMessage::new("Invalid request token".to_string()));
        }
        let user_id = get_user_id_from_token(&message.token);
        let response_data = self.database.user_roles.by_user(user_id)?;

        Ok(Message::new_with_serialize(GET_ROLES, response_data))
    }

    fn handle_create_role(&self, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Create Role");
        let message = CreateRoleRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
            .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let role = self.database.roles.create_role(
            message.name,
            message.description,
            message.permission,
        )?;

        Ok(Message::new_with_serialize(CREATE_ROLE, role))
    }
}
