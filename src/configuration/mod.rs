mod database;

pub use database::DatabaseConfig;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub application_port: u16,
}

impl Config {
    pub fn init() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .add_source(config::File::with_name("config"))
            .build()?
            .try_deserialize::<Self>()
    }
}
