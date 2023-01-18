use reqwest::Url;
use secrecy::Secret;
use serde::{de, Deserialize};

use crate::domain::SubscriberEmail;

#[derive(Clone, Debug, Deserialize)]
pub struct EmailClientSettings {
    pub base_url: UrlWrapper,
    pub sender: SubscriberEmail,
    pub authorization_token: Secret<String>,
}

#[derive(Clone, Debug)]
pub struct UrlWrapper(Url);

impl<'de> Deserialize<'de> for UrlWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Url::parse(&String::deserialize(deserializer)?)
            .map_err(de::Error::custom)
            .map(UrlWrapper)
    }
}

impl Into<Url> for UrlWrapper {
    fn into(self) -> Url {
        self.0
    }
}
