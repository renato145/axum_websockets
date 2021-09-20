// use actix_web::rt::task::JoinHandle;
// use tracing::{subscriber::set_global_default, Subscriber};
// use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
// use tracing_log::LogTracer;
// use tracing_subscriber::{
//     fmt::MakeWriter, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry,
// };

// /// Compose multiple layers into a tracing subscriber
// pub fn get_subscriber(
//     name: String,
//     env_filter: String,
//     sink: impl MakeWriter + Send + Sync + 'static,
// ) -> impl Subscriber + Send + Sync {
//     let env_filter =
//         EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
//     let formatting_layer = BunyanFormattingLayer::new(name, sink);
//     Registry::default()
//         .with(env_filter)
//         .with(JsonStorageLayer)
//         .with(formatting_layer)
// }

// /// Register a subscriber as a global default to proces span data.
// ///
// /// It should only be called once!
// pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
//     LogTracer::init().expect("Failed to set logger.");
//     set_global_default(subscriber).expect("Failed to set subscriber.");
// }

// pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
// where
//     F: FnOnce() -> R + Send + 'static,
//     R: Send + 'static,
// {
//     let current_span = tracing::Span::current();
//     actix_web::rt::task::spawn_blocking(move || current_span.in_scope(f))
// }
