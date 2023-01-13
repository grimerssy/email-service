mod health_check;
mod subscriptions;

use std::net::TcpListener;

use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;
use zero2prod::configuration::{Config, DatabaseConfig};

struct Server {
    config: Config,
    address: String,
    db_name: String,
    db_pool: PgPool,
}

impl Server {
    async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
        let port = listener.local_addr().unwrap().port();
        let config = Config::init().expect("Failed to initialize config.");
        let db_name = Uuid::new_v4().to_string();
        let db_pool = Self::create_database(&config.database, db_name.clone()).await;
        let server = zero2prod::run(listener, db_pool.clone()).expect("Failed to bind address.");
        let _ = tokio::spawn(server);
        Self {
            config,
            address: format!("http://127.0.0.1:{}", port),
            db_name,
            db_pool,
        }
    }

    async fn create_database(config: &DatabaseConfig, db_name: String) -> PgPool {
        PgConnection::connect(&config.url())
            .await
            .expect("Failed to connect to the database.")
            .execute(format!(r#"create database "{}";"#, db_name).as_str())
            .await
            .expect("Failed to create database.");

        let db_url = format!("{}/{}", config.url_no_db(), db_name);
        let pool = PgPool::connect(&db_url)
            .await
            .expect("Failed to connect to the database.");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations on the database.");

        pool
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        let db_name = self.db_name.clone();
        let config = self.config.clone();

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let mut conn = PgConnection::connect(&config.database.url())
                    .await
                    .expect("Failed to connect to Postgres");

                conn.execute(&*format!(
                    "select pg_terminate_backend(pg_stat_activity.pid)
                    from pg_stat_activity
                    where datname = '{}'
                      and pid <> pg_backend_pid();",
                    db_name
                ))
                .await
                .expect("Failed to disconnect other sessions");

                conn.execute(format!(r#"drop database "{}";"#, db_name).as_str())
                    .await
                    .expect("Failed to drop temporary database: {}");

                let _ = tx.send(());
            })
        });
        let _ = rx.recv();
    }
}