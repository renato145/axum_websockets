use axum_websockets::{
    configuration::get_configuration,
    telemetry::{get_subscriber, init_subscriber},
    Application,
};

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let subscriber = get_subscriber(
        "actix_websockets".into(),
        "info".into(),
        std::io::stdout,
        configuration.console,
    );
    init_subscriber(subscriber);

    let application = Application::build(configuration).expect("Failed to build application.");
    application.run_until_stopped().await?;
    Ok(())
}
