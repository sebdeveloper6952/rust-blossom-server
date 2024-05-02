#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub env: String,
    pub host: String,
    pub port: u16,
    pub db: DatabaseConfig,
    pub telemetry: TelemetryConfig,
    pub cdn: CdnConfig,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct TelemetryConfig {
    pub kind: TelemetryKind,
    pub uptrace_dsn: String,
    pub service_name: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct CdnConfig {
    pub base_url: String,
    pub whitelisted_pubkeys: Vec<String>,
    pub max_upload_size_bytes: u64,
    pub min_upload_size_bytes: u64,
    pub allowed_mime_types: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub enum TelemetryKind {
    Stdout,
    Uptrace,
    None,
}

impl From<&str> for TelemetryKind {
    fn from(val: &str) -> Self {
        match val {
            "stdout" => Self::Stdout,
            "uptrace" => Self::Uptrace,
            _ => Self::None,
        }
    }
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
