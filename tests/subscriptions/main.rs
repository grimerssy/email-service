mod post;

use async_trait::async_trait;
use reqwest::{Client, Response};
use test_server::TestServer;

fn endpoint(base: &str) -> String {
    format!("{}/subscriptions", base)
}

#[async_trait]
trait Server {
    async fn post_supscriptions(&self, body: String) -> Response;
}

#[async_trait]
impl Server for TestServer {
    async fn post_supscriptions(&self, body: String) -> Response {
        Client::new()
            .post(endpoint(&self.addr))
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}
