use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    handler::get,
    response::IntoResponse,
    routing::BoxRoute,
    Router, Server,
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::configuration::{Settings, WebsocketSettings};
// use actix::Actor;
// use actix_web::{
//     dev::Server,
//     web::{self, Data},
//     App, HttpServer,
// };
use std::net::{SocketAddr, TcpListener};
// use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    app: Router<BoxRoute>,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", configuration.host, configuration.port);
        // let listener = SocketAddr::new(configuration.host, configuration.port);

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let app = build_app(listener, configuration.websocket)?;
        // Ok(Self { port, server })
        Ok(Self { port, app })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
    //     self.server.await
    // }
}

pub fn build_app(
    listener: TcpListener,
    websocket_settings: WebsocketSettings,
) -> Result<Router<BoxRoute>, std::io::Error> {
    tracing::info!("{:?}", websocket_settings);
    let a = DefaultMakeSpan::default().include_headers(true);
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(a))
        .boxed();
    Ok(app)
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
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
