#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub allowed_origins: Vec<String>,
    pub log_format: LogFormat,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid LOG_FORMAT {0:?}: expected \"pretty\" or \"json\"")]
    InvalidLogFormat(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/dev.db".to_string());
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:4563".to_string());
        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|origins| {
                origins
                    .split(',')
                    .map(str::trim)
                    .filter(|origin| !origin.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let log_format = match std::env::var("LOG_FORMAT")
            .unwrap_or_else(|_| "pretty".to_string())
            .as_str()
        {
            "pretty" => LogFormat::Pretty,
            "json" => LogFormat::Json,
            other => return Err(ConfigError::InvalidLogFormat(other.to_string())),
        };

        Ok(Self {
            database_url,
            bind_addr,
            allowed_origins,
            log_format,
        })
    }
}
