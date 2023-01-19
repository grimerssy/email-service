use crate::Server;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use zero2prod::{configuration::DatabaseConfig, telemetry};

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

impl Server {
    pub async fn create_database(config: &DatabaseConfig) -> PgPool {
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
