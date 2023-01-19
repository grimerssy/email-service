mod get;

use std::str;

use crate::TestServer;
use reqwest::{Client, Response};

fn endpoint(base: &str) -> String {
    format!("{}/health_check", base)
}

impl TestServer {
    pub async fn get_health_check(&self) -> Response {
        Client::new()
            .get(endpoint(&self.addr))
            .send()
            .await
            .expect("Failed to execute request")
    }
}
