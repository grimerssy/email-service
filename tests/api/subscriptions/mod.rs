use crate::{mock_email_server, Server, TestServer};
use linkify::{LinkFinder, LinkKind};
use wiremock::ResponseTemplate;

#[macros::test]
async fn post_returns_200_for_valid_data(server: TestServer) {
    mock_email_server(ResponseTemplate::new(200), None)
        .mount(&server.email_server)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    let response = server.post_subscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[macros::test]
async fn post_persists_the_new_subscriber(server: TestServer) {
    mock_email_server(ResponseTemplate::new(200), None)
        .mount(&server.email_server)
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
    mock_email_server(ResponseTemplate::new(200), None)
        .mount(&server.email_server)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    server.post_subscriptions(body.into()).await;
}

#[macros::test]
async fn post_sends_an_email_with_confirmation_link(server: TestServer) {
    mock_email_server(ResponseTemplate::new(200), None)
        .mount(&server.email_server)
        .await;
    let body = "name=John%20Doe&email=example%40gmail.com";
    server.post_subscriptions(body.into()).await;

    let body = server
        .email_server
        .received_requests()
        .await
        .unwrap()
        .first()
        .cloned()
        .unwrap()
        .body;
    let body = serde_json::from_slice::<serde_json::Value>(&body).unwrap();

    let get_link = |s: &str| {
        let links = LinkFinder::new()
            .links(s)
            .filter(|l| l.kind() == &LinkKind::Url)
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 1);
        links.first().unwrap().as_str().to_owned()
    };
    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, text_link);
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
