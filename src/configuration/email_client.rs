use reqwest::Url;
use secrecy::Secret;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::time::Duration;

use crate::domain::SubscriberEmail;

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct EmailClientConfig {
    pub timeout: Duration,
    #[serde_as(as = "DisplayFromStr")]
    pub base_url: Url,
    pub sender: SubscriberEmail,
    pub authorization_token: Secret<String>,
}
