use std::collections::HashMap;

use crate::{
    Endpoints, Links, ServerExt, TestServer, TestUser,
    FAILED_TO_EXECUTE_REQUEST,
};
use hashmap_macro::hashmap;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[macros::test]
async fn newsletters_are_delievered_to_confirmed_subscribers(
    server: TestServer,
) {
    create_confirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(1))
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;
    let response = server.post_admin_newsletters(&newsletter_body()).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
}

#[macros::test]
async fn newsletters_are_not_delievered_to_unconfirmed_subscribers(
    server: TestServer,
) {
    create_unconfirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(0))
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;
    let response = server.post_admin_newsletters(&newsletter_body()).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
}

#[macros::test]
async fn post_with_no_authorization_header_is_rejected(server: TestServer) {
    let response = server
        .http_client
        .post(server.admin_newsletters())
        .json(&newsletter_body())
        .send()
        .await
        .expect(FAILED_TO_EXECUTE_REQUEST);
    assert_eq!(response.status().as_u16(), 303);
}

#[macros::test]
async fn nonexistent_users_are_rejected(server: TestServer) {
    let user = TestUser::new();
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;
    let response = server.post_admin_newsletters(&newsletter_body()).await;
    assert_eq!(response.status().as_u16(), 303);
}

#[macros::test]
async fn unauthenticated_users_are_rejected(server: TestServer) {
    let response = server.post_admin_newsletters(&newsletter_body()).await;
    assert_eq!(response.status().as_u16(), 303);
}

#[macros::test]
async fn post_newsletters_returns_400_for_invalid_data(server: TestServer) {
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;
    let test_cases = vec![
        (
            hashmap!(
                "textContent" => "Newsletter body",
                "htmlContent" => "<p>Newsletter body</p>",
            ),
            "missing title",
        ),
        (hashmap!("title" => "New newsletter!"), "missing content"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = server.post_admin_newsletters(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API does not return 400 Bad Request when the payload is {error_message}."
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

fn newsletter_body() -> HashMap<&'static str, &'static str> {
    hashmap!(
             "title" => "Newsletter title",
             "textContent" => "Newsletter body as plain text",
             "htmlContent" => "<p>Newsletter body as HTML</p>",
    )
}
