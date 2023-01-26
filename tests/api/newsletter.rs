use crate::{Helpers, Links, Server, TestServer};
use hashmap_macro::hashmap;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[macros::test]
async fn newsletters_are_not_delievered_to_unconfirmed_subscribers(server: TestServer) {
    create_unconfirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(0))
        .await;
    let newsletter_request_body = serde_json::json!({
             "title": "Newsletter title",
             "content": {
                 "text": "Newsletter body as plain text",
                 "html": "<p>Newsletter body as HTML</p>",
             }
    });
    let response = server.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[macros::test]
async fn newsletters_are_delievered_to_confirmed_subscribers(server: TestServer) {
    create_confirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(1))
        .await;
    let newsletter_request_body = serde_json::json!({
             "title": "Newsletter title",
             "content": {
                 "text": "Newsletter body as plain text",
                 "html": "<p>Newsletter body as HTML</p>",
             }
    });
    let response = server.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[macros::test]
async fn post_newsletters_returns_400_for_invalid_data(server: TestServer) {
    let test_cases = vec![
        (
            serde_json::json!({
            "content": {
                "text": "Newsletter body",
                "html": "<p>Newsletter body</p>",
            }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "New newsletter!"}),
            "missing content",
        ),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = server.post_newsletters(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API does not return 400 Bad Request when the payload is {}.",
            error_message
        );
    }
}

async fn create_unconfirmed_subscriber(server: &TestServer) -> Links {
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&server.email_server)
        .await;
    let body = hashmap!["name" => "John Doe", "email" => "example@gmail.com"];
    server
        .post_subscriptions(&body)
        .await
        .error_for_status()
        .unwrap();

    let email_request = server
        .email_server
        .received_requests()
        .await
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    server.extract_links(&email_request)
}

async fn create_confirmed_subscriber(server: &TestServer) {
    let links = create_unconfirmed_subscriber(server).await;
    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
