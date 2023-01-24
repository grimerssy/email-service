use dotenvy::dotenv;
use once_cell::sync::Lazy;
use reqwest::Url;
use std::time::Duration;
use wiremock::MockServer;
pub use zero2prod::DbPool;
use zero2prod::{configuration::Config, telemetry, Server};

pub static TELEMETRY: Lazy<Result<(), String>> = Lazy::new(|| {
    let (name, filter) = ("test", "debug");
    if std::env::var("TEST_LOG")
        .unwrap_or_default()
        .parse::<bool>()
        .unwrap_or_default()
    {
        telemetry::init(name, filter, std::io::stdout)
    } else {
        telemetry::init(name, filter, std::io::sink)
    }
});

pub struct TestServer {
    pub base_url: String,
    pub port: u16,
    pub db_pool: DbPool,
    pub email_server: MockServer,
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
            c.database.options.database = db_pool.connect_options().get_database().unwrap().into();
            c.email_client.timeout = Duration::from_millis(200);
            c.email_client.base_url = Url::parse(&email_server.uri()).unwrap();
            c
        };
        let server = Server::build(config.clone()).expect("Failed to run server.");
        let base_url = config.application.base_url.clone();
        let port = server.port();
        let _ = tokio::spawn(server.run());
        Self {
            base_url,
            port,
            db_pool,
            email_server,
        }
    }
}
