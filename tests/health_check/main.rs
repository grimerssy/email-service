mod get;

use async_trait::async_trait;
use reqwest::{Client, Response};
use test_server::TestServer;

fn endpoint(base: &str) -> String {
    format!("{}/health_check", base)
}

#[async_trait]
trait Server {
    async fn get_health_check(&self) -> Response;
}

#[async_trait]
impl Server for TestServer {
    async fn get_health_check(&self) -> Response {
        Client::new()
            .get(endpoint(&self.addr))
            .send()
            .await
            .expect("Failed to execute request")
    }
}
