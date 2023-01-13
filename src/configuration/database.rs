use std::str::FromStr;

use serde::{de, Deserialize, Deserializer};

#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(rename = "url")] // reads from DATABASE_URL env var
    opts: DbConnectionOptions,
}

#[derive(Clone)]
struct DbConnectionOptions {
    host: String,
    port: u16,
    username: String,
    password: String,
    name: String,
}

impl DatabaseConfig {
    pub fn url_no_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.opts.username, self.opts.password, self.opts.host, self.opts.port
        )
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.url_no_db(), self.opts.name)
    }
}

impl FromStr for DbConnectionOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use regex::Regex;
        let rg = Regex::new(r"postgres://(.+):(.+)@(.+):(.+)/(.+)").unwrap();
        let captures = rg
            .captures(s)
            .ok_or("Database URL did not match the expected format.")?;
        Ok(DbConnectionOptions {
            username: captures.get(1).unwrap().as_str().into(),
            password: captures.get(2).unwrap().as_str().into(),
            host: captures.get(3).unwrap().as_str().into(),
            port: captures
                .get(4)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| "Invalid port value.")?,
            name: captures.get(5).unwrap().as_str().into(),
        })
    }
}

impl<'de> Deserialize<'de> for DbConnectionOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let url = &String::deserialize(deserializer)?;
        Self::from_str(url).map_err(de::Error::custom)
    }
}
