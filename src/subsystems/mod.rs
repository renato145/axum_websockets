pub mod python_repo;

use crate::message::{ResultMessage, TaskMessage, WebsocketMessage};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsocketSystem {
    PythonRepo,
}

#[async_trait::async_trait]
trait Subsystem {
    type Error;
    type Task;

    fn system(&self) -> WebsocketSystem;

    async fn handle_message(
        &self,
        task: Self::Task,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, Self::Error>;

    #[tracing::instrument(name = "Handling subsystem message", skip(self, rx))]
    async fn handle_messages(
        &self,
        mut rx: oneshot::Receiver<TaskMessage>,
        sender: mpsc::Sender<WebsocketMessage>,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        Self::Task: DeserializeOwned + Send,
        Self::Error: std::error::Error,
    {
        loop {
            match (&mut rx).await {
                Ok(msg) => {
                    let result = match serde_json::from_str::<Self::Task>(&msg.name) {
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
                Err(_) => {
                    tracing::info!("Sender dropped.");
                    return Ok(());
                }
            }
        }
    }
}
