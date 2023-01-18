mod application;
mod database;
mod email_client;
mod environment;

use application::ApplicationConfig;
pub use database::DatabaseConfig;
pub use email_client::EmailClientSettings;
use environment::Environment;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub email_client: EmailClientSettings,
    pub application: ApplicationConfig,
}

impl Config {
    pub fn init() -> Result<Self, config::ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        let config_directory = base_path.join("config");
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT");
        let config_file = format!("{}.yaml", environment.as_str());
        config::Config::builder()
            .add_source(config::File::from(config_directory.join("base.yaml")))
            .add_source(config::File::from(config_directory.join(config_file)))
            .add_source(config::Environment::default().separator("_"))
            .build()?
            .try_deserialize::<Self>()
    }
}
