use axum::{
    extract::{Extension, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    AddExtensionLayer, Router,
};
use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

use crate::{
    configuration::{Settings, WebsocketSettings},
    websocket::handle_socket,
};
use std::{net::TcpListener, sync::Arc};

pub struct Application {
    listener: TcpListener,
    port: u16,
    app: Router,
}

impl Application {
    pub fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        // let listener = SocketAddr::new(configuration.ip, configuration.port);
        let address = format!("{}:{}", configuration.host, configuration.port);
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr()?.port();
        let app = build_app(configuration.websocket);
        Ok(Self {
            listener,
            port,
            app,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        axum::Server::from_tcp(self.listener)?
            .serve(self.app.into_make_service())
            .await
    }
}

fn build_app(websocket_settings: WebsocketSettings) -> Router {
    tracing::info!("{:?}", websocket_settings);
    let websocket_settings = Arc::new(websocket_settings);

    Router::new()
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
        .layer(AddExtensionLayer::new(websocket_settings))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(websocket_settings): Extension<Arc<WebsocketSettings>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, websocket_settings))
}
