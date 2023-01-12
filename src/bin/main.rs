use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::Config;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = Config::try_init().expect("Failed to initialize config.");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.application_port))
        .expect("Failed to bind address");
    let db_pool = PgPool::connect(&config.database.url())
        .await
        .expect("Failed to connect to the database.");
    zero2prod::run(listener, db_pool)?.await
}
