mod confirm;

use crate::TestServer;
use hashmap_macro::hashmap;
use wiremock::ResponseTemplate;
use zero2prod::DbPool;

#[sqlx::test]
async fn post_returns_200_for_valid_data(pool: DbPool) {
    let server = TestServer::run(pool).await;
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = hashmap!["name" => "John Doe", "email" => "example@gmail.com"];
    let response = server.post_subscriptions(&body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[sqlx::test]
async fn post_persists_the_new_subscriber(pool: DbPool) {
    let server = TestServer::run(pool).await;
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = hashmap!["name" => "John Doe", "email" => "example@gmail.com"];
    server.post_subscriptions(&body).await;

    let saved = sqlx::query!(r"select email, name, status from subscriptions",)
        .fetch_all(&server.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.len(), 1);

    let saved = saved.first().unwrap();
    assert_eq!(saved.name, "John Doe");
    assert_eq!(saved.email, "example@gmail.com");
    assert_eq!(saved.status, "pending_confirmation");
}

#[sqlx::test]
async fn post_sends_an_email_for_valid_data(pool: DbPool) {
    let server = TestServer::run(pool).await;
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = hashmap!["name" => "John Doe", "email" => "example@gmail.com"];
    server.post_subscriptions(&body).await;
}

#[sqlx::test]
async fn post_sends_an_email_with_confirmation_link(pool: DbPool) {
    let server = TestServer::run(pool).await;
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = hashmap!["name" => "John Doe", "email" => "example@gmail.com"];
    server.post_subscriptions(&body).await;

    let email_request = server
        .email_server
        .received_requests()
        .await
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    let links = server.extract_links(&email_request);
    assert_eq!(links.text, links.html);
}

#[sqlx::test]
async fn post_returns_400_when_data_is_missing(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let cases = vec![
        (hashmap!["name" => "John Doe"], "form is missing the email"),
        (
            hashmap!["email" => "example@gmail.com"],
            "form is missing the name",
        ),
        (hashmap![], "form is missing both name and email"),
    ];
    for (invalid_body, reason) in cases {
        let response = server.post_subscriptions(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}

#[sqlx::test]
async fn post_returns_400_when_data_is_invalid(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let cases = vec![
        (
            hashmap!["name" => "", "email" => "example@gmail.com"],
            "has empty name",
        ),
        (
            hashmap!["name" => "John Doe", "email" => ""],
            "has empty email",
        ),
        (
            hashmap!["name" => "John Doe", "email" => "definitely-not-an-email"],
            "has invalid email",
        ),
    ];
    for (invalid_body, reason) in cases {
        let response = server.post_subscriptions(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}
