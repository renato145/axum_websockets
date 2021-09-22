use axum_websockets::{
    configuration::get_configuration,
    telemetry::{get_subscriber, init_subscriber},
    Application,
};

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let subscriber = get_subscriber("actix_websockets".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(configuration);
    application.run_until_stopped().await?;
    Ok(())
}
