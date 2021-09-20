use super::{
    message::{ClientMessage, Connect, SubSystemPart, TaskMessage, TaskPayload, WebsocketSystems},
    subsystem::WebsocketSubSystem,
};
use crate::error_chain_fmt;
use actix::{Actor, AsyncContext, Handler, Message, Recipient};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, thread};
use systemstat::Platform;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum PcUsageError {
    #[error("Invalid path: {0:?}")]
    InvalidPath(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PcUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl SubSystemPart for Result<serde_json::Value, PcUsageError> {
    fn system(&self) -> Option<WebsocketSystems> {
        Some(WebsocketSystems::PcUsage)
    }
}

pub struct PcUsageSystem {
    sessions: HashMap<Uuid, Recipient<ClientMessage>>,
}

impl Default for PcUsageSystem {
    fn default() -> Self {
        Self {
            sessions: Default::default(),
        }
    }
}

impl WebsocketSubSystem for PcUsageSystem {
    type Error = PcUsageError;
    type Task = Tasks;

    fn get_address(&self, id: &Uuid) -> Option<&Recipient<ClientMessage>> {
        self.sessions.get(id)
    }

    fn system(&self) -> WebsocketSystems {
        WebsocketSystems::PcUsage
    }
}

impl Actor for PcUsageSystem {
    type Context = actix::Context<Self>;
}

impl Handler<Connect> for PcUsageSystem {
    type Result = ();

    #[tracing::instrument(name = "Connecting socket to PcUsageSystem", skip(self, _ctx))]
    fn handle(&mut self, message: Connect, _ctx: &mut Self::Context) -> Self::Result {
        self.sessions.insert(message.id, message.addr);
    }
}

/// Dispatcher for task handlers
impl Handler<TaskMessage> for PcUsageSystem {
    type Result = ();

    #[tracing::instrument(name = "Handle task (PcUsageSystem)", skip(self, ctx))]
    fn handle(&mut self, task_message: TaskMessage, ctx: &mut Self::Context) -> Self::Result {
        let task = match self.task_from_message(task_message.clone()) {
            Ok(task) => task,
            Err(_) => return,
        };

        let addr = ctx.address();
        match task {
            Tasks::CpuLoad => addr.do_send(GetCpuLoad::from(task_message.payload)),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tasks {
    CpuLoad,
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct GetCpuLoad {
    id: Uuid,
}

impl From<TaskPayload> for GetCpuLoad {
    fn from(payload: TaskPayload) -> Self {
        Self { id: payload.id }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuLoadResult {
    pub user: f32,
    pub system: f32,
}

impl Handler<GetCpuLoad> for PcUsageSystem {
    type Result = ();

    #[tracing::instrument(name = "Handle task GetCpuLoad", skip(self, _ctx))]
    fn handle(&mut self, message: GetCpuLoad, _ctx: &mut Self::Context) -> Self::Result {
        let sys = systemstat::System::new();
        let result = sys
            .cpu_load()
            .context("Failed to read cpu load.")
            .map(|cpu| {
                thread::sleep(std::time::Duration::from_millis(200)); // TODO: This blocks the server
                cpu.done().context("Failed to read cpu load.").map(|cpu| {
                    let result = cpu
                        .iter()
                        .map(|cpu_load| CpuLoadResult {
                            user: cpu_load.user,
                            system: cpu_load.system,
                        })
                        .collect::<Vec<_>>();
                    serde_json::to_value(result).context("Failed to serialize cpu result.")
                })
            })
            .and_then(std::convert::identity)
            .and_then(std::convert::identity)
            .map_err(PcUsageError::UnexpectedError);

        self.send_message(message.id, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::message::RawWebsocketMessage;

    #[test]
    fn correctly_deserialize_task_name() {
        let message = serde_json::json!({
            "system": "pc_usage",
            "task": "cpu_load",
        });
        let message = serde_json::from_value::<RawWebsocketMessage>(message).unwrap();
        let task = serde_json::from_str::<Tasks>(&format!("{:?}", message.task)).unwrap();
        assert_eq!(Tasks::CpuLoad, task);
    }
}
