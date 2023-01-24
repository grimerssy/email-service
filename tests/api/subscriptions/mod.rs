mod confirm;

use crate::{Helpers, Server, TestServer};
use wiremock::ResponseTemplate;

#[macros::test]
async fn post_returns_200_for_valid_data(server: TestServer) {
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    let response = server.post_subscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[macros::test]
async fn post_persists_the_new_subscriber(server: TestServer) {
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    server.post_subscriptions(body.into()).await;

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

#[macros::test]
async fn post_sends_an_email_for_valid_data(server: TestServer) {
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    server.post_subscriptions(body.into()).await;
}

#[macros::test]
async fn post_sends_an_email_with_confirmation_link(server: TestServer) {
    server
        .mock_email_server(ResponseTemplate::new(200), None)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    server.post_subscriptions(body.into()).await;

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

#[macros::test]
async fn post_returns_400_when_data_is_missing(server: TestServer) {
    let cases = vec![
        ("name=John%20Doe", "form is missing the email"),
        ("email=example%40gmail.com", "form is missing the name"),
        ("", "form is missing both name and email"),
    ];
    for (invalid_body, reason) in cases {
        let response = server.post_subscriptions(invalid_body.into()).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}

#[macros::test]
async fn post_returns_400_when_data_is_invalid(server: TestServer) {
    let cases = vec![
        ("name=&email=example%40gmail.com", "has empty name"),
        ("name=John%20Doe&email=", "has empty email"),
        (
            "name=John%20Doe&email=definitely-not-an-email",
            "has invalid email",
        ),
    ];
    for (invalid_body, reason) in cases {
        let response = server.post_subscriptions(invalid_body.into()).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}
