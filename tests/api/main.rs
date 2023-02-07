mod admin;
mod health_check;
mod login;
mod newsletter;
mod subscriptions;

use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher,
    Version,
};
use dotenvy::dotenv;
use hashmap_macro::hashmap;
use once_cell::sync::Lazy;
use reqwest::{header::LOCATION, redirect::Policy, Client, Response, Url};
use std::collections::HashMap;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::Config,
    issue_delivery::{try_execute_task, ExecutionOutcome},
    telemetry, DbPool, EmailClient, Server,
};

static FAILED_TO_EXECUTE_REQUEST: &str = "Failed to execute request";

pub static TELEMETRY: Lazy<Result<(), String>> = Lazy::new(|| {
    let (name, filter) = ("test", "debug");
    if std::env::var("TEST_LOG").is_ok() {
        telemetry::init(name, filter, std::io::stdout)
    } else {
        telemetry::init(name, filter, std::io::sink)
    }
});

pub struct TestServer {
    pub base_url: String,
    pub port: u16,
    pub http_client: Client,
    pub db_pool: DbPool,
    pub email_server: MockServer,
    pub email_client: EmailClient,
}

impl TestServer {
    pub async fn run(db_pool: DbPool) -> Self {
        dotenv().ok();
        Lazy::force(&TELEMETRY)
            .as_ref()
            .expect("Failed to initialize telemetry");

        let email_server = MockServer::start().await;
        let config = {
            let mut c = Config::init().expect("Failed to initialize config");
            c.application.port = 0;
            c.database.options.database =
                db_pool.connect_options().get_database().unwrap().into();
            c.email_client.base_url = Url::parse(&email_server.uri()).unwrap();
            c
        };
        let email_client = EmailClient::new(config.email_client.clone());
        let server = Server::build(config.clone())
            .await
            .expect("Failed to run server");
        let base_url = config.application.base_url;
        let port = server.port();
        let http_client = Client::builder()
            .redirect(Policy::none())
            .cookie_store(true)
            .build()
            .unwrap();
        #[allow(clippy::let_underscore_future)]
        let _ = tokio::spawn(server.run());
        Self {
            base_url,
            port,
            http_client,
            db_pool,
            email_server,
            email_client,
        }
    }
}

impl TestServer {
    async fn get_health_check(&self) -> Response {
        self.http_client
            .get(self.health_check())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_login(&self) -> Response {
        self.http_client
            .get(self.login())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_login(&self, body: &HashMap<&str, &str>) -> Response {
        self.http_client
            .post(self.login())
            .form(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_admin_logout(&self) -> Response {
        self.http_client
            .post(self.admin_logout())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_admin_dashboard(&self) -> Response {
        self.http_client
            .get(self.admin_dashboard())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_admin_password(&self) -> Response {
        self.http_client
            .get(self.admin_password())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_admin_password(
        &self,
        body: &HashMap<&str, &str>,
    ) -> Response {
        self.http_client
            .post(self.admin_password())
            .form(body)
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
            .get(self.subscriptions_confirm())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn get_admin_newsletters(&self) -> Response {
        self.http_client
            .get(self.admin_newsletters())
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }

    async fn post_admin_newsletters(
        &self,
        body: &HashMap<&str, &str>,
    ) -> Response {
        self.http_client
            .post(self.admin_newsletters())
            .form(body)
            .send()
            .await
            .expect(FAILED_TO_EXECUTE_REQUEST)
    }
}

struct Links {
    text: Url,
    html: Url,
}

impl TestServer {
    fn when_sending_an_email(&self) -> wiremock::MockBuilder {
        use wiremock::{
            matchers::{method, path},
            Mock,
        };
        Mock::given(path("/email")).and(method("post"))
    }

    async fn mock_email_server(
        &self,
        response: wiremock::ResponseTemplate,
        expect: Option<u64>,
    ) {
        let builder = self.when_sending_an_email().respond_with(response);
        if let Some(requests) = expect {
            builder.expect(requests)
        } else {
            builder
        }
        .mount(&self.email_server)
        .await
    }

    async fn dispatch_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
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
        let body =
            serde_json::from_slice::<serde_json::Value>(&request.body).unwrap();
        let (text, html) = {
            let mut links = ["Text", "Html"].iter().map(|x| {
                let mut link =
                    extract_link(body[format!("{x}Body")].as_str().unwrap());
                link.set_port(Some(self.port)).unwrap();
                link
            });
            (links.next().unwrap(), links.next().unwrap())
        };
        Links { text, html }
    }

    fn assert_is_redirect_to(
        &self,
        response: &reqwest::Response,
        location: &str,
    ) {
        assert_eq!(response.status().as_u16(), 303);
        assert_eq!(response.headers().get(LOCATION).unwrap(), location);
    }
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

    async fn login(&self, server: &TestServer) -> Response {
        let body = hashmap!(
            "username" => self.username.as_str(),
            "password" => self.password.as_str(),
        );
        server.post_login(&body).await
    }
}

impl TestServer {
    fn addr(&self) -> String {
        format!("{}:{}", self.base_url, self.port)
    }

    fn health_check(&self) -> String {
        format!("{}/health_check", self.addr())
    }

    fn login(&self) -> String {
        format!("{}/login", self.addr())
    }

    fn admin_logout(&self) -> String {
        format!("{}/logout", self.admin())
    }

    fn admin(&self) -> String {
        format!("{}/admin", self.addr())
    }

    fn admin_dashboard(&self) -> String {
        format!("{}/dashboard", self.admin())
    }

    fn admin_password(&self) -> String {
        format!("{}/password", self.admin())
    }

    fn subscriptions(&self) -> String {
        format!("{}/subscriptions", self.addr())
    }

    fn subscriptions_confirm(&self) -> String {
        format!("{}/confirm", self.subscriptions())
    }

    fn admin_newsletters(&self) -> String {
        format!("{}/newsletters", self.admin())
    }
}
