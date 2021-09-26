use serde::{Deserialize, Serialize};

/// Internal messages.
#[derive(Debug)]
pub enum WebsocketMessage {
    Ping(Vec<u8>),
    Close,
}

/// Messages to send to client.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    // pub system: WebsocketSystems,
    pub success: bool,
    pub payload: serde_json::Value,
}
