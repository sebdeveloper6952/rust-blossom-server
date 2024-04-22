#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub env: String,
    pub host: String,
    pub port: u16,
    pub db: DatabaseConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct TelemetryConfig {
    pub uptrace_dsn: String,
    pub service_name: String,
}

#[tracing::instrument]
pub fn get_config() -> Result<Config, config::ConfigError> {
    let base_path = std::env::current_dir().expect("config failed to read base path");
    let cfg_dir = base_path.join("config");
    let cfg = config::Config::builder()
        .add_source(config::File::from(cfg_dir.join("config.yml")))
        .build()?;

    cfg.try_deserialize::<Config>()
}
