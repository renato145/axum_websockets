use crate::subsystems::WebsocketSystem;
use serde::{Deserialize, Serialize};

/// Internal messages.
#[derive(Debug)]
pub enum WebsocketMessage {
    TaskResult(ResultMessage),
    Ping(Vec<u8>),
    Close,
}

/// Message from client.
#[derive(Deserialize)]
pub struct ClientMessage {
    pub system: WebsocketSystem,
    pub task: String,
    #[serde(default = "serde_json::Value::default")]
    pub payload: serde_json::Value,
}

#[derive(Debug)]
pub struct TaskMessage {
    pub name: String,
    pub payload: serde_json::Value,
}

impl From<ClientMessage> for TaskMessage {
    fn from(msg: ClientMessage) -> Self {
        Self {
            name: msg.task,
            payload: msg.payload,
        }
    }
}

/// Messages to send to client.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultMessage {
    pub system: Option<WebsocketSystem>,
    pub success: bool,
    pub payload: serde_json::Value,
}

impl ResultMessage {
    pub fn from_json(payload: serde_json::Value, system: Option<WebsocketSystem>) -> Self {
        Self {
            system,
            success: true,
            payload,
        }
    }

    pub fn from_error<E: ToString>(e: E, system: Option<WebsocketSystem>) -> Self {
        let payload = serde_json::Value::String(e.to_string());
        Self {
            system,
            success: false,
            payload,
        }
    }
}
