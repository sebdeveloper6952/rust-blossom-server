use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub(crate) fn init_stdout_tracing(service_name: String) -> Result<(), String> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("TRACE"));
    let formatting_layer = BunyanFormattingLayer::new(service_name, std::io::stdout);

    let sub = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    LogTracer::init().expect("failed to init LogTracer");
    set_global_default(sub).expect("failed to register global tracing subscriber");

    Ok(())
}
