use std::net::TcpListener;

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
    let email_client = EmailClient::new(config.email_client);

    zero2prod::run(listener, db_pool, email_client)?.await
}
