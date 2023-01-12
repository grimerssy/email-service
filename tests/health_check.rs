use std::net::TcpListener;

use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{Config, DatabaseConfig};

#[tokio::test]
async fn it_works() {
    let app = mock_app().await;

    let url = format!("{}/health_check", app.address);
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_data() {
    let app = mock_app().await;

    let url = format!("{}/subscriptions", app.address);
    let body = "name=John%20Doe&email=example%40gmail.com";
    let response = reqwest::Client::new()
        .post(url)
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!(r"select email, name from subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "John Doe");
    assert_eq!(saved.email, "example@gmail.com");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let app = mock_app().await;

    let url = format!("{}/subscriptions", app.address);
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=John%20Doe", "form is missing the email"),
        ("email=example%40gmail.com", "form is missing the name"),
        ("", "form is missing both name and email"),
    ];
    for (invalid_body, reason) in test_cases {
        let response = client
            .post(url.clone())
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "Server does not return 400 when form {reason}.",
        );
    }
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn mock_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let config = Config::try_init().expect("Failed to initialize config.");
    let db_pool = mock_database(&config.database).await;
    let server = zero2prod::run(listener, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool,
    }
}

async fn mock_database(config: &DatabaseConfig) -> PgPool {
    let db_name = Uuid::new_v4();
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
