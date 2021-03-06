use crate::{
    configuration::WebsocketSettings,
    error::WebsocketError,
    message::{ClientMessage, ResultMessage, TaskMessage, WebsocketMessage},
    subsystems::{
        pc_usage::PcUsageSystem, python_repo::PythonRepoSystem, Subsystem, WebsocketSystem,
    },
    telemetry::tokio_spawn,
};
use anyhow::Context;
use axum::extract::ws::{Message, WebSocket};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc;

pub struct Session {
    hb: Mutex<Instant>,
    settings: WebsocketSettings,
}

impl Session {
    pub fn new(settings: &WebsocketSettings) -> Self {
        Session {
            hb: Mutex::new(Instant::now()),
            settings: settings.clone(),
        }
    }

    /// Sends ping to client every x seconds.
    /// Also checks heartbeats from client.
    #[tracing::instrument(name = "Heartbeat task", level = "trace", skip(self, sender))]
    async fn hb(&self, sender: mpsc::Sender<WebsocketMessage>) -> Result<(), WebsocketError> {
        let mut interval = tokio::time::interval(self.settings.heartbeat_interval);
        loop {
            interval.tick().await;
            // Check client heartbeats
            if Instant::now().duration_since(*self.hb.lock().unwrap())
                > self.settings.client_timeout
            {
                // Heartbeat timed out
                tracing::info!("Websocket client heartbeat failed, disconnecting.");
                sender.send(WebsocketMessage::Close).await?;
                return Ok(());
            }
            // Send ping
            tracing::trace!("Sending ping.");
            sender.send(WebsocketMessage::Ping(vec![])).await?;
        }
    }
}

#[tracing::instrument(name = "Handling websocket message", skip(socket, settings))]
pub async fn handle_socket(socket: WebSocket, settings: Arc<WebsocketSettings>) {
    let session = Arc::new(Session::new(&settings));
    let (socket_sender, socket_receiver) = socket.split();
    let (tx, rx) = mpsc::channel(32);

    let mut recv_task = tokio_spawn(receive_message(rx, socket_sender));
    let mut hb_task = tokio_spawn({
        let tx = tx.clone();
        let session = session.clone();
        async move { session.hb(tx).await }
    });

    let python_repo_system = PythonRepoSystem {};
    let (python_repo_tx, python_repo_rx) = mpsc::channel(32);
    let mut python_repo_task = tokio_spawn({
        let tx = tx.clone();
        async move { python_repo_system.handle_messages(python_repo_rx, tx).await }
    });

    let pc_usage_system = PcUsageSystem {};
    let (pc_usage_tx, pc_usage_rx) = mpsc::channel(32);
    let mut pc_usage_task = tokio_spawn({
        let tx = tx.clone();
        async move { pc_usage_system.handle_messages(pc_usage_rx, tx).await }
    });

    let mut client_recv_task = tokio_spawn({
        let session = session.clone();
        let tx = tx.clone();
        async move {
            client_receive_task(socket_receiver, session, tx, python_repo_tx, pc_usage_tx).await
        }
    });

    let (result, _, _) = futures::future::select_all(vec![
        &mut client_recv_task,
        &mut recv_task,
        &mut python_repo_task,
        &mut pc_usage_task,
        &mut hb_task,
    ])
    .await;

    match result {
        Ok(Err(e)) => tracing::info!("Got WebsocketError: {:?}", e),
        Err(e) => tracing::info!("Got JoinError: {:?}", e),
        _ => {}
    }
}

#[tracing::instrument(
    name = "Client receiver task",
    level = "trace",
    skip(socket_receiver, session, sender, python_repo_tx)
)]
async fn client_receive_task(
    mut socket_receiver: SplitStream<WebSocket>,
    session: Arc<Session>,
    sender: mpsc::Sender<WebsocketMessage>,
    python_repo_tx: mpsc::Sender<TaskMessage>,
    pc_usage_tx: mpsc::Sender<TaskMessage>,
) -> Result<(), WebsocketError> {
    while let Some(msg) = socket_receiver.next().await {
        match msg {
            Err(e) => tracing::info!("Client disconnected: {:?}", e),
            Ok(msg) => {
                tracing::trace!("Received: {:?}", msg);
                match msg {
                    Message::Text(msg) => match serde_json::from_str::<ClientMessage>(&msg) {
                        Ok(msg) => {
                            let tx = match msg.system {
                                WebsocketSystem::PythonRepo => &python_repo_tx,
                                WebsocketSystem::PcUsage => &pc_usage_tx,
                            };
                            tx.send(msg.into()).await?;
                        }
                        Err(e) => {
                            tracing::info!("Failed to deserialize message: {:?}", e);
                            sender
                                .send(WebsocketMessage::TaskResult(ResultMessage::from_error(
                                    e, None,
                                )))
                                .await?;
                        }
                    },
                    Message::Binary(_) => {
                        tracing::info!("Invalid binary message from client.");
                    }
                    Message::Ping(msg) => {
                        *session.hb.lock().unwrap() = Instant::now();
                        sender.send(WebsocketMessage::Ping(msg)).await?;
                    }
                    Message::Pong(_) => {
                        *session.hb.lock().unwrap() = Instant::now();
                    }
                    Message::Close(_) => todo!(),
                }
            }
        }
    }
    Ok(())
}

#[tracing::instrument(
    name = "Internal receiver task",
    level = "trace"
    skip(rx, socket_sender),
)]
async fn receive_message(
    mut rx: mpsc::Receiver<WebsocketMessage>,
    mut socket_sender: SplitSink<WebSocket, Message>,
) -> Result<(), WebsocketError> {
    while let Some(msg) = rx.recv().await {
        tracing::trace!("Received: {:?}", msg);
        match msg {
            WebsocketMessage::Ping(msg) => {
                socket_sender
                    .send(Message::Ping(msg))
                    .await
                    .context("Failed to send Ping message to socket.")?;
            }
            WebsocketMessage::Close => {
                socket_sender
                    .send(Message::Close(None))
                    .await
                    .context("Failed to send Close message to socket.")?;
                break;
            }
            WebsocketMessage::TaskResult(msg) => {
                let msg =
                    serde_json::to_string(&msg).context("Failed to serialize ClientMessage")?;
                socket_sender
                    .send(Message::Text(msg))
                    .await
                    .context("Failed to send ClientMessage to socket.")?;
            }
        }
    }
    Ok(())
}
