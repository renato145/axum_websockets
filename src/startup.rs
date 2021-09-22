use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    handler::get,
    response::IntoResponse,
    routing::BoxRoute,
    Router,
};
use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

use crate::configuration::{Settings, WebsocketSettings};
use std::net::SocketAddr;

pub struct Application {
    listener: SocketAddr,
    port: u16,
    app: Router<BoxRoute>,
}

impl Application {
    pub fn build(configuration: Settings) -> Self {
        let listener = SocketAddr::new(configuration.ip, configuration.port);
        let port = listener.port();
        let app = build_app(configuration.websocket);
        Self {
            listener,
            port,
            app,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        axum::Server::bind(&self.listener)
            .serve(self.app.into_make_service())
            .await
    }
}

fn build_app(websocket_settings: WebsocketSettings) -> Router<BoxRoute> {
    tracing::info!("{:?}", websocket_settings);
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(
            // More on TraceLayer: https://docs.rs/tower-http/0.1.1/tower_http/trace/index.html
            TraceLayer::new_for_http()
                .make_span_with(tracing::Span::current())
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Micros),
                ),
        )
        .boxed();
    app
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            println!("Client says: {:?}", msg);
        } else {
            println!("client disconnected");
            return;
        }
    }

    loop {
        if socket
            .send(Message::Text(String::from("Hi!")))
            .await
            .is_err()
        {
            println!("client disconnected");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_secs(7)).await;
    }
}
