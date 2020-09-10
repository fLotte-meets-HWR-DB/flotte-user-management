use super::rpc_methods::*;
use crate::database::Database;
use crate::server::messages::{ErrorMessage, InfoEntry, ValidateTokenRequest};
use msgrpc::message::Message;
use msgrpc::server::RpcServer;
use rmp_serde::Deserializer;
use serde::Deserialize;
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
                GET_ROLES => unimplemented!(),
                VALIDATE_TOKEN => self.handle_validate_token(&handler.message.data),
                _ => Err(ErrorMessage::new("Invalid Method".to_string())),
            }
            .unwrap_or_else(|e| Message::new_with_serialize(ERROR, e));
            log::trace!("Responding with {:?}", &response);
            handler.done(response);
        }
    }

    fn handle_validate_token(&self, data: &Vec<u8>) -> RpcResult<Message> {
        log::trace!("Validating token.");
        let message =
            ValidateTokenRequest::deserialize(&mut Deserializer::new(&mut data.as_slice()))
                .map_err(|e| ErrorMessage::new(e.to_string()))?;
        let valid = self
            .database
            .users
            .validate_request_token(&message.token)
            .unwrap_or(false);
        log::trace!("Serializing...");
        let data = rmp_serde::to_vec(&valid).map_err(|e| ErrorMessage::new(e.to_string()))?;

        Ok(Message::new(VALIDATE_TOKEN, data))
    }

    fn handle_info(&self) -> RpcResult<Message> {
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
            ],
        ))
    }
}
