use secrecy::{ExposeSecret, Secret};
use serde::{de, Deserialize, Deserializer};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    #[serde(rename = "url")] // reads from DATABASE_URL env var
    pub options: ConnectOptions,
    pub require_ssl: bool,
}

#[derive(Clone, Debug)]
pub struct ConnectOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Secret<String>,
    pub database: String,
}

impl DatabaseConfig {
    fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.options.host)
            .port(self.options.port)
            .username(&self.options.username)
            .password(self.options.password.expose_secret())
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.options.database)
    }

    pub fn with_default_db(&self) -> PgConnectOptions {
        self.without_db().database("postgres")
    }
}

impl FromStr for ConnectOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use regex::Regex;
        let rg = Regex::new(r"postgres://(.+):(.+)@(.+):(.+)/(.+)").unwrap();
        let captures = rg
            .captures(s)
            .ok_or("Database URL did not match the expected format")?;
        Ok(ConnectOptions {
            username: captures.get(1).unwrap().as_str().into(),
            password: Secret::new(captures.get(2).unwrap().as_str().into()),
            host: captures.get(3).unwrap().as_str().into(),
            port: captures
                .get(4)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| "Failed to parse port number")?,
            database: captures.get(5).unwrap().as_str().into(),
        })
    }
}

impl<'de> Deserialize<'de> for ConnectOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let url = &String::deserialize(deserializer)?;
        Self::from_str(url).map_err(de::Error::custom)
    }
}
