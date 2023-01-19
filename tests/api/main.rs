mod health_check;
mod helpers;
mod subscriptions;

use std::{net::TcpListener, time::Duration};

use dotenvy::dotenv;
use helpers::TELEMETRY;
use once_cell::sync::Lazy;
use sqlx::PgPool;
use uuid::Uuid;
use zero2prod::{configuration::Config, EmailClient};

#[derive(Debug)]
struct Server {
    config: Config,
    db_pool: PgPool,
}

impl Server {
    async fn init() -> Self {
        dotenv().ok();
        Lazy::force(&TELEMETRY)
            .as_ref()
            .expect("Failed to initialize telemetry");

        let mut config = Config::init().expect("Failed to initialize config");

        let listener = TcpListener::bind(format!("{}:0", config.application.host))
            .expect("Failed to bind random port");
        config.application.port = listener.local_addr().unwrap().port();
        let database = Uuid::new_v4().to_string();
        config.database.options.database = database;

        let db_pool = Self::create_database(&config.database).await;
        config.email_client.timeout = Duration::from_millis(200);
        let email_client = EmailClient::new(config.email_client.clone());

        let server = zero2prod::run(listener, db_pool.clone(), email_client)
            .expect("Failed to bind address");
        let _ = tokio::spawn(server);

        Self { config, db_pool }
    }
}
