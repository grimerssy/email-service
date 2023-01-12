mod database;

pub use database::DatabaseConfig;
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub application_port: u16,
}

impl Config {
    pub fn try_init() -> Result<Self, config::ConfigError> {
        dotenv().ok();
        config::Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .add_source(config::File::with_name("config"))
            .build()?
            .try_deserialize::<Self>()
    }
}
