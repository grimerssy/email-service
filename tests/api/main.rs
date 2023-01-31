mod health_check;
mod newsletter;
mod subscriptions;

use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use async_trait::async_trait;
use reqwest::{Response, Url};
use std::collections::HashMap;
use test_server::TestServer;
use uuid::Uuid;
use zero2prod::DbPool;

static FAILED_TO_EXECUTE_REQUEST: &str = "Failed to execute request";

#[async_trait]
trait ServerExt {
    async fn get_health_check(&self) -> Response;
    async fn post_subscriptions(&self, body: &HashMap<&str, &str>) -> Response;
    async fn get_subscriptions_confirm(&self) -> Response;
    async fn post_newsletters(&self, user: TestUser, body: &serde_json::Value) -> Response;

    async fn mock_email_server(&self, response: wiremock::ResponseTemplate, expect: Option<u64>);
    fn extract_links(&self, email_request: &wiremock::Request) -> Links;
}

#[async_trait]
impl ServerExt for TestServer {
    async fn get_health_check(&self) -> Response {
        self.http_client
            .get(self.health_check())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_subscriptions(&self, body: &HashMap<&str, &str>) -> Response {
        self.http_client
            .post(self.subscriptions())
            .form(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_subscriptions_confirm(&self) -> Response {
        self.http_client
            .post(self.subscriptions())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_newsletters(&self, user: TestUser, body: &serde_json::Value) -> Response {
        self.http_client
            .post(self.newsletters())
            .basic_auth(user.username, Some(user.password))
            .json(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
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
                let mut link = extract_link(body[format!("{x}Body")].as_str().unwrap());
                link.set_port(Some(self.port)).unwrap();
                link
            });
            (links.next().unwrap(), links.next().unwrap())
        };
        Links { text, html }
    }
}

struct Links {
    text: Url,
    html: Url,
}

#[derive(Clone, Debug)]
struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    fn new() -> Self {
        let user_id = Uuid::new_v4();
        let username = user_id.to_string();
        let password = username.clone();
        Self {
            user_id,
            username,
            password,
        }
    }

    async fn stored(pool: &DbPool) -> Self {
        let user = Self::new();
        user.store(pool).await;
        user
    }

    async fn store(&self, pool: &DbPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            r#"
        insert into users (user_id, username, password_hash)
        values ($1, $2, $3);
        "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user");
    }
}

trait Endpoints {
    fn addr(&self) -> String;
    fn health_check(&self) -> String;
    fn subscriptions(&self) -> String;
    fn subscriptions_confirm(&self) -> String;
    fn newsletters(&self) -> String;
}

impl Endpoints for TestServer {
    fn addr(&self) -> String {
        format!("{}:{}", self.base_url, self.port)
    }

    fn health_check(&self) -> String {
        format!("{}/health_check", self.addr())
    }

    fn subscriptions(&self) -> String {
        format!("{}/subscriptions", self.addr())
    }

    fn subscriptions_confirm(&self) -> String {
        format!("{}/confirm", self.subscriptions())
    }

    fn newsletters(&self) -> String {
        format!("{}/newsletters", self.addr())
    }
}
