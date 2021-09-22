use super::error::WebsocketError;
use actix::{Message, Recipient};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Raw message from clients.
#[derive(Debug, Deserialize)]
pub struct RawWebsocketMessage {
    pub system: WebsocketSystems,
    pub task: String,
    #[serde(default = "serde_json::Value::default")]
    pub payload: serde_json::Value,
}

/// Messages accepted from server.
#[derive(Debug)]
pub struct WebsocketMessage {
    pub system: WebsocketSystems,
    pub task: TaskMessage,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebsocketSystems {
    PythonRepo,
    PcUsage,
}

/// Messages that represent tasks.
#[derive(Debug, Clone, actix::Message)]
#[rtype(result = "()")]
pub struct TaskMessage {
    pub name: String,
    pub payload: TaskPayload,
}

#[derive(Debug, Clone)]
pub struct TaskPayload {
    pub id: Uuid,
    pub data: serde_json::Value,
}

impl WebsocketMessage {
    pub fn parse(id: Uuid, message: &str) -> Result<Self, WebsocketError> {
        let raw = serde_json::from_str::<RawWebsocketMessage>(message)
            .context("Failed to deserialize message.")
            .map_err(WebsocketError::MessageParseError)?;

        let message = Self {
            system: raw.system,
            task: TaskMessage {
                name: raw.task,
                payload: TaskPayload {
                    id,
                    data: raw.payload,
                },
            },
        };

        Ok(message)
    }
}

/// Messages to send to client.
#[derive(Debug, Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub system: Option<WebsocketSystems>,
    pub success: bool,
    pub payload: serde_json::Value,
}

pub trait SubSystemPart {
    fn system(&self) -> Option<WebsocketSystems>;
}

pub trait ClientMessager: SubSystemPart {
    fn success(&self) -> bool;
    fn payload(self) -> serde_json::Value;
    fn to_message(self) -> ClientMessage
    where
        Self: Sized,
    {
        ClientMessage {
            system: self.system(),
            success: self.success(),
            payload: self.payload(),
        }
    }
}

impl<E> ClientMessager for Result<serde_json::Value, E>
where
    Result<serde_json::Value, E>: SubSystemPart,
    E: std::error::Error,
{
    fn success(&self) -> bool {
        self.is_ok()
    }

    fn payload(self) -> serde_json::Value {
        match self {
            Ok(value) => value,
            Err(e) => e.to_string().into(),
        }
    }
}

/// Start connection with a server.
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub id: Uuid,
    pub addr: Recipient<ClientMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_deserialize_websocket_message() {
        let message = serde_json::json!({
            "system": "python_repo",
            "task": "some_task",
        });
        let message = serde_json::from_value::<RawWebsocketMessage>(message).unwrap();
        assert_eq!(WebsocketSystems::PythonRepo, message.system);
    }
}
