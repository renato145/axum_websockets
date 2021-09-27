pub mod python_repo;

use crate::{
    error::WebsocketError,
    message::{ResultMessage, TaskMessage, WebsocketMessage},
};
use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebsocketSystem {
    PythonRepo,
}

#[async_trait::async_trait]
pub trait Subsystem {
    type Error;
    type Task;

    fn system(&self) -> WebsocketSystem;

    async fn handle_message(
        &self,
        task: Self::Task,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, Self::Error>;

    #[tracing::instrument(
        name = "Handling subsystem message",
        skip(self, internal_receiver, sender)
    )]
    async fn handle_messages(
        &self,
        mut internal_receiver: mpsc::Receiver<TaskMessage>,
        sender: mpsc::Sender<WebsocketMessage>,
    ) -> Result<(), WebsocketError>
    where
        Self: Sized,
        Self::Task: DeserializeOwned + Send,
        Self::Error: std::error::Error,
    {
        while let Some(msg) = internal_receiver.recv().await {
            tracing::debug!("Received: {:?}", msg);
            let result = match serde_json::from_str::<Self::Task>(&format!("{:?}", msg.name))
                .context("Failed to deserialize message.")
            {
                Ok(task) => match self.handle_message(task, msg.payload).await {
                    Ok(res) => ResultMessage::from_json(res, Some(self.system())),
                    Err(e) => ResultMessage::from_error(e, Some(self.system())),
                },
                Err(e) => ResultMessage::from_error(e, Some(self.system())),
            };
            if sender
                .send(WebsocketMessage::TaskResult(result))
                .await
                .is_err()
            {
                tracing::info!("Websocket receiver dropped.");
            }
        }
        Ok(())
    }
}
