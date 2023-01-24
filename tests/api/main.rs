mod health_check;
mod subscriptions;

use async_trait::async_trait;
use reqwest::{header::CONTENT_TYPE, Client, Response};
use test_server::TestServer;

static FORM_URLENCODED: &str = "application/x-www-form-urlencoded";
static FAILED_TO_EXECUTE: &str = "Failed to execute request";

#[async_trait]
trait Server {
    async fn get_health_check(&self) -> Response;

    async fn post_subscriptions(&self, body: String) -> Response;
    async fn get_subscriptions_confirm(&self) -> Response;
}

#[async_trait]
impl Server for TestServer {
    async fn get_health_check(&self) -> Response {
        Client::new()
            .get(health_check(&self.addr))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }

    async fn post_subscriptions(&self, body: String) -> Response {
        Client::new()
            .post(subscriptions(&self.addr))
            .header(CONTENT_TYPE, FORM_URLENCODED)
            .body(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }

    async fn get_subscriptions_confirm(&self) -> Response {
        Client::new()
            .get(subscriptions_confirm(&self.addr))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }
}

pub fn mock_email_server(
    response: wiremock::ResponseTemplate,
    expect: Option<u64>,
) -> wiremock::Mock {
    use wiremock::{
        matchers::{method, path},
        Mock,
    };
    let builder = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(response);
    if let Some(requests) = expect {
        builder.expect(requests)
    } else {
        builder
    }
}

fn health_check(base: &str) -> String {
    format!("{}/health_check", base)
}

fn subscriptions(base: &str) -> String {
    format!("{}/subscriptions", base)
}

fn subscriptions_confirm(base: &str) -> String {
    format!("{}/confirm", subscriptions(base))
}
