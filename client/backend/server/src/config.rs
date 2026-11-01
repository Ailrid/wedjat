use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConversionConfig {
    pub max_concurrent_jobs: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataConfig {
    pub source_root: String,
    pub output_root: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub conversion: ConversionConfig,
    pub data: DataConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, config::ConfigError> {
        let builder =
            config::Config::builder().add_source(config::File::with_name(path).required(true));

        let settings = builder.build()?;
        settings.try_deserialize()
    }
}
