use std::{net::TcpListener, time::Duration};

use dotenvy::dotenv;
use sqlx::PgPool;
use zero2prod::{telemetry, Config, EmailClient};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    telemetry::init("zero2prod", "info", std::io::stdout).expect("Failed to initialize telemetry");

    let config = Config::init().expect("Failed to initialize config");
    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.application.host, config.application.port
    ))
    .expect("Failed to bind address");
    let db_pool = PgPool::connect_lazy_with(config.database.with_db());
    let email_client = EmailClient::new(
        Duration::from_secs(5),
        config.email_client.base_url,
        config.email_client.sender,
        config.email_client.authorization_token,
    );

    zero2prod::run(listener, db_pool, email_client)?.await
}
