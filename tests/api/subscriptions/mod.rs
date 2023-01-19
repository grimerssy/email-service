mod post;

use crate::TestServer;
use reqwest::{Client, Response};

fn endpoint(base: &str) -> String {
    format!("{}/subscriptions", base)
}

impl TestServer {
    pub async fn post_supscriptions(&self, body: String) -> Response {
        Client::new()
            .post(endpoint(&self.addr))
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}
