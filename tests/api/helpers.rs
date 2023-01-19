use dotenvy::dotenv;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;
use zero2prod::{
    configuration::{Config, DatabaseConfig},
    telemetry, Server,
};

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

#[derive(Debug)]
pub struct TestServer {
    pub addr: String,
    pub db_pool: PgPool,
}

impl TestServer {
    pub async fn init() -> Self {
        dotenv().ok();
        Lazy::force(&TELEMETRY)
            .as_ref()
            .expect("Failed to initialize telemetry");

        let config = {
            let mut c = Config::init().expect("Failed to initialize config");
            c.application.port = 0;
            c.database.options.database = Uuid::new_v4().to_string();
            c.email_client.timeout = Duration::from_millis(200);
            c
        };
        let db_pool = Self::create_database(&config.database).await;
        let server = Server::build(config.clone()).expect("Failed to run server.");
        let addr = format!("http://{}:{}", config.application.host, server.port());
        let _ = tokio::spawn(server.run());
        Self { addr, db_pool }
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

impl Drop for TestServer {
    fn drop(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        let connect_options = self.db_pool.connect_options().clone();
        let database_name = connect_options.get_database().unwrap().to_owned();
        let connect_options = connect_options.database("postgres");
        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let mut conn = PgConnection::connect_with(&connect_options)
                    .await
                    .expect("Failed to connect to Postgres");

                conn.execute(&*format!(
                    "select pg_terminate_backend(pg_stat_activity.pid)
                    from pg_stat_activity
                    where datname = '{}'
                      and pid <> pg_backend_pid();",
                    database_name
                ))
                .await
                .expect("Failed to disconnect other sessions");

                conn.execute(format!(r#"drop database "{}";"#, database_name).as_str())
                    .await
                    .expect("Failed to drop temporary database: {}");

                let _ = tx.send(());
            })
        });
        let _ = rx.recv();
    }
}
