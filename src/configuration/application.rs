use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Clone, Debug, Deserialize)]
pub struct ApplicationConfig {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

impl ApplicationConfig {
    pub fn addr(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}
