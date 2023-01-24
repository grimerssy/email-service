mod health_check;
mod subscriptions;

use async_trait::async_trait;
use reqwest::{header::CONTENT_TYPE, Client, Response, Url};
use test_server::TestServer;

static FAILED_TO_EXECUTE: &str = "Failed to execute request";

static FORM_URLENCODED: &str = "application/x-www-form-urlencoded";

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
            .get(health_check(&self.addr()))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }

    async fn post_subscriptions(&self, body: String) -> Response {
        Client::new()
            .post(subscriptions(&self.addr()))
            .header(CONTENT_TYPE, FORM_URLENCODED)
            .body(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }

    async fn get_subscriptions_confirm(&self) -> Response {
        Client::new()
            .get(subscriptions_confirm(&self.addr()))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE)
    }
}

struct Links {
    text: Url,
    html: Url,
}

#[async_trait]
trait Helpers {
    fn addr(&self) -> String;
    async fn mock_email_server(&self, response: wiremock::ResponseTemplate, expect: Option<u64>);
    fn extract_links(&self, email_request: &wiremock::Request) -> Links;
}

#[async_trait]
impl Helpers for TestServer {
    fn addr(&self) -> String {
        format!("{}:{}", self.base_url, self.port)
    }

    async fn mock_email_server(&self, response: wiremock::ResponseTemplate, expect: Option<u64>) {
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
        .mount(&self.email_server)
        .await
    }

    fn extract_links(&self, request: &wiremock::Request) -> Links {
        use linkify::{LinkFinder, LinkKind};
        let extract_link = |s: &str| {
            let links = LinkFinder::new()
                .links(s)
                .filter(|l| l.kind() == &LinkKind::Url)
                .collect::<Vec<_>>();
            assert_eq!(links.len(), 1);
            let link = links.first().unwrap().as_str().to_owned();
            Url::parse(&link).unwrap()
        };
        let body = serde_json::from_slice::<serde_json::Value>(&request.body).unwrap();
        let text = extract_link(body["TextBody"].as_str().unwrap());
        let html = extract_link(body["HtmlBody"].as_str().unwrap());
        Links { text, html }
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
