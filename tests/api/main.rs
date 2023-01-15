mod health_check;
mod subscriptions;

use std::net::TcpListener;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;
use zero2prod::{
    configuration::{Config, DatabaseConfig},
    telemetry,
};

static TELEMETRY: Lazy<Result<(), String>> = Lazy::new(|| {
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

        let server = zero2prod::run(listener, db_pool.clone()).expect("Failed to bind address");
        let _ = tokio::spawn(server);

        Self { config, db_pool }
    }

    async fn create_database(config: &DatabaseConfig) -> PgPool {
        PgConnection::connect_with(&config.with_default_db())
            .await
            .expect("Failed to connect to the database")
            .execute(format!(r#"create database "{}";"#, config.options.database).as_str())
            .await
            .expect("Failed to create database");

        let pool = PgPool::connect_with(config.with_db())
            .await
            .expect("Failed to connect to the database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations on the database");

        pool
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        let database = self.config.database.options.database.clone();
        let config = self.config.clone();

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let mut conn = PgConnection::connect_with(&config.database.with_default_db())
                    .await
                    .expect("Failed to connect to Postgres");

                conn.execute(&*format!(
                    "select pg_terminate_backend(pg_stat_activity.pid)
                    from pg_stat_activity
                    where datname = '{}'
                      and pid <> pg_backend_pid();",
                    database
                ))
                .await
                .expect("Failed to disconnect other sessions");

                conn.execute(format!(r#"drop database "{}";"#, database).as_str())
                    .await
                    .expect("Failed to drop temporary database: {}");

                let _ = tx.send(());
            })
        });
        let _ = rx.recv();
    }
}
