use tracing::{subscriber::set_global_default, Instrument, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::MakeWriter, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry,
};

/// Compose multiple layers into a tracing subscriber
pub fn get_subscriber<W: for<'a> MakeWriter<'a> + Send + Sync + 'static>(
    name: String,
    mut env_filter: String,
    sink: W,
    tokio_console: bool,
) -> Box<dyn Subscriber + Send + Sync> {
    if tokio_console {
        env_filter.push_str(",tokio=trace,runtime=trace");
    }

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    let registry = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    if tokio_console {
        // spawn the console server in the background, returning a `Layer`:
        let console_layer = console_subscriber::spawn();
        Box::new(registry.with(console_layer))
    } else {
        Box::new(registry)
    }
}

/// Register a subscriber as a global default to proces span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}

pub fn tokio_spawn<T>(future: T) -> tokio::task::JoinHandle<T::Output>
where
    T: std::future::Future + Send + 'static,
    T::Output: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::spawn(future.instrument(current_span))
}
