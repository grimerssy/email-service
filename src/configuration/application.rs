use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct ApplicationConfig {
    pub host: String,
    pub port: u16,
}
