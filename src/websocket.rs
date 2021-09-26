use crate::{configuration::WebsocketSettings, error_chain_fmt};
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
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum WebsocketError {
    #[error("Channel closed.")]
    MpscSendError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for WebsocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl From<mpsc::error::SendError<WebsocketMessage>> for WebsocketError {
    fn from(_: mpsc::error::SendError<WebsocketMessage>) -> Self {
        WebsocketError::MpscSendError
    }
}

pub struct Session {
    id: Uuid,
    hb: Mutex<Instant>,
    settings: WebsocketSettings,
}

impl Session {
    pub fn new(settings: &WebsocketSettings) -> Self {
        Session {
            id: Uuid::new_v4(),
            hb: Mutex::new(Instant::now()),
            settings: settings.clone(),
        }
    }

    /// Sends ping to client every x seconds.
    /// Also checks heartbeats from client.
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
            tracing::debug!("Sending ping...");
            sender.send(WebsocketMessage::Ping(vec![])).await?;
        }
    }
}

enum WebsocketMessage {
    Ping(Vec<u8>),
    Close,
}

#[tracing::instrument(
	name = "Handling websocket message",
	skip(socket, settings),
	// fields(message=tracing::field::Empty)
)]
pub async fn handle_socket(socket: WebSocket, settings: Arc<WebsocketSettings>) {
    let session = Arc::new(Session::new(&settings));
    let (socket_sender, socket_receiver) = socket.split();
    let (tx, rx) = mpsc::channel(32);

    let mut client_recv_task = tokio::spawn({
        let session = session.clone();
        let tx = tx.clone();
        async move { client_receive_task(socket_receiver, session, tx).await }
    });
    let mut recv_task = tokio::spawn(async move { receive_message(rx, socket_sender).await });
    let mut hb_task = tokio::spawn(async move { session.hb(tx).await });

    let result = tokio::select! {
        a = (&mut client_recv_task) => a,
        b = (&mut recv_task) => b,
        c = (&mut hb_task) => c
    };

    match result {
        Ok(Err(e)) => tracing::info!("Got WebsocketError: {:?}", e),
        Err(e) => tracing::info!("Got JoinError: {:?}", e),
        _ => {}
    }
}

async fn client_receive_task(
    mut socket_receiver: SplitStream<WebSocket>,
    session: Arc<Session>,
    sender: mpsc::Sender<WebsocketMessage>,
) -> Result<(), WebsocketError> {
    while let Some(msg) = socket_receiver.next().await {
        match msg {
            Err(e) => tracing::info!("Client disconnected: {:?}", e),
            Ok(msg) => match msg {
                Message::Text(msg) => tracing::info!("Received: {:?}", msg),
                Message::Binary(_) => {
                    tracing::info!("Invalid binary message from client.");
                }
                Message::Ping(msg) => {
                    tracing::debug!("Received Ping from client.");
                    *session.hb.lock().unwrap() = Instant::now();
                    sender.send(WebsocketMessage::Ping(msg)).await?;
                }
                Message::Pong(_) => {
                    tracing::debug!("Received Pong from client.");
                    *session.hb.lock().unwrap() = Instant::now();
                }
                Message::Close(_) => todo!(),
            },
        }
    }
    Ok(())
}

async fn receive_message(
    mut rx: mpsc::Receiver<WebsocketMessage>,
    mut socket_sender: SplitSink<WebSocket, Message>,
) -> Result<(), WebsocketError> {
    while let Some(msg) = rx.recv().await {
        match msg {
            WebsocketMessage::Ping(msg) => {
                socket_sender
                    .send(Message::Ping(msg))
                    .await
                    .context("Failed to send ping to socket")?;
            }
            WebsocketMessage::Close => {
                break;
            }
        }
    }
    Ok(())
}
