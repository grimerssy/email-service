use crate::{Links, TestServer, TestUser, FAILED_TO_EXECUTE_REQUEST};
use fake::{faker::lorem::en::Sentence, Fake};
use hashmap_macro::hashmap;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};
use zero2prod::DbPool;

#[sqlx::test]
async fn newsletter_delivery_is_idempotent(pool: DbPool) {
    let server = TestServer::run(pool).await;
    create_confirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(1))
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = server.get_admin_newsletters().await.text().await.unwrap();
    assert!(html_page.contains(
        "<p><i>You have successfully published a newsletter.</i></p>"
    ));

    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = server.get_admin_newsletters().await.text().await.unwrap();
    assert!(html_page.contains(
        "<p><i>You have successfully published a newsletter.</i></p>"
    ));
    server.dispatch_pending_emails().await;
}

#[sqlx::test]
async fn concurrent_requests_are_handled(pool: DbPool) {
    let server = TestServer::run(pool).await;
    create_confirmed_subscriber(&server).await;
    server
        .mock_email_server(
            ResponseTemplate::new(200).set_delay(Duration::from_millis(500)),
            Some(1),
        )
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let b = body(&idempotency_key);
    let response1 = server.post_admin_newsletters(&b);
    let response2 = server.post_admin_newsletters(&b);
    let (response1, response2) = tokio::join!(response1, response2);
    assert_eq!(response1.status().as_u16(), 303);
    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    server.dispatch_pending_emails().await;
}

#[sqlx::test]
async fn newsletters_are_delievered_to_confirmed_subscribers(pool: DbPool) {
    let server = TestServer::run(pool).await;
    create_confirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(1))
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
    server.dispatch_pending_emails().await;
}

#[sqlx::test]
async fn newsletters_are_not_delievered_to_unconfirmed_subscribers(
    pool: DbPool,
) {
    let server = TestServer::run(pool).await;
    create_unconfirmed_subscriber(&server).await;
    server
        .mock_email_server(ResponseTemplate::new(200), Some(0))
        .await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    server.assert_is_redirect_to(&response, "/admin/newsletters");
}

#[sqlx::test]
async fn post_with_no_authorization_header_is_rejected(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server
        .http_client
        .post(server.admin_newsletters())
        .json(&body(&idempotency_key))
        .send()
        .await
        .expect(FAILED_TO_EXECUTE_REQUEST);
    assert_eq!(response.status().as_u16(), 303);
}

#[sqlx::test]
async fn nonexistent_users_are_rejected(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::new();
    user.login(&server).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    assert_eq!(response.status().as_u16(), 303);
}

#[sqlx::test]
async fn unauthenticated_users_are_rejected(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let idempotency_key = Uuid::new_v4().to_string();
    let response = server.post_admin_newsletters(&body(&idempotency_key)).await;
    assert_eq!(response.status().as_u16(), 303);
}

#[sqlx::test]
async fn post_newsletters_returns_400_for_invalid_data(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;
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
    use fake::faker::internet::en::SafeEmail;
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&server.email_server)
        .await;
    let name = Sentence(2..3).fake::<String>();
    let email = SafeEmail().fake::<String>();
    let body = hashmap!["name" => name.as_str(), "email" => email.as_str()];
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
        .pop()
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

fn body(idempotency_key: &str) -> HashMap<&str, &str> {
    hashmap!(
        "title" => "Newsletter title",
        "textContent" => "Newsletter body as plain text",
        "htmlContent" => "<p>Newsletter body as HTML</p>",
        "idempotencyKey" => idempotency_key
    )
}
