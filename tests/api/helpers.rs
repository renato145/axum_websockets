use actix_web_actors::ws;
use actix_websockets::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
    websocket::message::ClientMessage,
};
use awc::Client;
use futures::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use std::time::Duration;

// Ensure that 'tracing' stack is only initialized once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
}

impl TestApp {
    pub async fn get_first_result(&self, message: &str) -> ClientMessage {
        let (_response, mut connection) = Client::new()
            .ws(format!("{}/ws/", self.address))
            .connect()
            .await
            .expect("Failed to connect to websocket.");

        connection
            .send(awc::ws::Message::Text(message.into()))
            .await
            .expect("Failed to send message.");

        loop {
            match connection.next().await {
                Some(Ok(ws::Frame::Text(msg))) => {
                    let msg = serde_json::from_slice::<ClientMessage>(&msg)
                        .expect(&format!("Failed to parse JSON: {:?}", msg));
                    tracing::info!("RESULT: {:?}", msg);
                    return msg;
                }
                Some(Ok(ws::Frame::Ping(_))) => {}
                err => {
                    tracing::error!("Receive message: {:?}", err);
                    panic!("Failed to receive message.");
                }
            }
        }
    }
}

pub async fn spawn_app() -> TestApp {
    // Set up tracing
    Lazy::force(&TRACING);

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Port 0 give us a random available port
        c.port = 0;
        c.websocket.heartbeat_interval = Duration::from_millis(50);
        c.websocket.client_timeout = Duration::from_millis(250);
        c
    };

    // Launch app as background task
    let application = Application::build(configuration)
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let test_app = TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
    };

    test_app
}
