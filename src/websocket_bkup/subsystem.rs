use super::message::{ClientMessage, ClientMessager, TaskMessage, WebsocketSystems};
use actix::Recipient;
use anyhow::Context;
use uuid::Uuid;

pub trait WebsocketSubSystem {
    type Error;
    type Task;

    fn system(&self) -> WebsocketSystems;

    fn get_address(&self, id: &Uuid) -> Option<&Recipient<ClientMessage>>;

    #[tracing::instrument(
		name = "Sending error message from SubSystem",
		skip(self),
		fields(subsystem=tracing::field::Empty)
	)]
    fn send_error(&self, id: Uuid, e: &Self::Error)
    where
        Self::Error: std::error::Error,
    {
        let type_name = std::any::type_name::<Self>();
        tracing::Span::current().record("subsystem", &tracing::field::debug(type_name));
        match self.get_address(&id) {
            Some(addr) => {
                let message = ClientMessage {
                    system: Some(self.system()),
                    success: false,
                    payload: e.to_string().into(),
                };
                if let Err(e) = addr.do_send(message) {
                    tracing::error!("Failed to send message from PythonRepoSystem: {:?}", e);
                }
            }
            None => {
                tracing::error!("No address found for id: {:?}", id);
            }
        }
    }

    #[tracing::instrument(
		name = "Sending message from SubSystem",
		skip(self),
		fields(subsystem=tracing::field::Empty)
	)]
    fn send_message(&self, id: Uuid, msg: Result<serde_json::Value, Self::Error>)
    where
        Result<serde_json::Value, Self::Error>: ClientMessager,
        Self::Error: std::error::Error,
    {
        let type_name = std::any::type_name::<Self>();
        tracing::Span::current().record("subsystem", &tracing::field::debug(type_name));
        match self.get_address(&id) {
            Some(addr) => {
                let message = msg.to_message();
                if let Err(e) = addr.do_send(message) {
                    tracing::error!("Failed to send message from PythonRepoSystem: {:?}", e);
                }
            }
            None => {
                tracing::error!("No address found for id: {:?}", id);
            }
        }
    }

    fn task_from_message(&self, task_message: TaskMessage) -> Result<Self::Task, Self::Error>
    where
        Self::Task: serde::de::DeserializeOwned,
        Self::Error: std::error::Error,
        anyhow::Error: Into<Self::Error>,
    {
        let task_name = format!("{:?}", task_message.name);
        let task = serde_json::from_str::<Self::Task>(&task_name)
            .context("Failed to deserialize task name.")
            .map_err(|e| e.into());

        if let Err(e) = &task {
            self.send_error(task_message.payload.id, e);
        }

        task
    }
}
