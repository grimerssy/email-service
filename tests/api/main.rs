mod health_check;
mod newsletter;
mod subscriptions;

use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::{Client, Response, Url};
use test_server::TestServer;

static FAILED_TO_EXECUTE_REQUEST: &str = "Failed to execute request";

#[async_trait]
trait Server {
    async fn get_health_check(&self) -> Response;

    async fn post_subscriptions(&self, body: &HashMap<&str, &str>) -> Response;
    async fn get_subscriptions_confirm(&self) -> Response;

    async fn post_newsletters(&self, body: &serde_json::Value) -> Response;
}

#[async_trait]
impl Server for TestServer {
    async fn get_health_check(&self) -> Response {
        Client::new()
            .get(health_check(&self.addr()))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_subscriptions(&self, body: &HashMap<&str, &str>) -> Response {
        Client::new()
            .post(subscriptions(&self.addr()))
            .form(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_subscriptions_confirm(&self) -> Response {
        Client::new()
            .get(subscriptions_confirm(&self.addr()))
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_newsletters(&self, body: &serde_json::Value) -> Response {
        Client::new()
            .post(newsletters(&self.addr()))
            .json(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
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
        let (text, html) = {
            let mut links = ["Text", "Html"].iter().map(|x| {
                let mut link = extract_link(body[format!("{}Body", x)].as_str().unwrap());
                link.set_port(Some(self.port)).unwrap();
                link
            });
            (links.next().unwrap(), links.next().unwrap())
        };
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

fn newsletters(base: &str) -> String {
    format!("{}/newsletters", base)
}
