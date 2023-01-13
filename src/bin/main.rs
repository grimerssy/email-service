use std::net::TcpListener;

use dotenvy::dotenv;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use zero2prod::{telemetry, Config};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    telemetry::init("zero2prod", "info", std::io::stdout).expect("Failed to initialize telemetry.");

    let config = Config::init().expect("Failed to initialize config.");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.application_port))
        .expect("Failed to bind address");
    let db_pool = PgPool::connect(config.database.url().expose_secret())
        .await
        .expect("Failed to connect to the database.");

    zero2prod::run(listener, db_pool)?.await
}
