use crate::TestServer;
use hashmap_macro::hashmap;
use wiremock::ResponseTemplate;
use zero2prod::DbPool;

#[sqlx::test]
async fn get_to_link_from_post_subscription_returns_200(pool: DbPool) {
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
    let link = server.extract_links(&email_request).html;
    assert_eq!(link.host_str(), Some("127.0.0.1"));

    let response = reqwest::get(link).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[sqlx::test]
async fn get_confirms_a_subscriber(pool: DbPool) {
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
    let mut link = server.extract_links(&email_request).html;
    assert_eq!(link.host_str(), Some("127.0.0.1"));
    link.set_port(Some(server.port)).unwrap();
    reqwest::get(link).await.unwrap();

    let saved = sqlx::query!(r"select status from subscriptions;")
        .fetch_one(&server.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions");
    assert_eq!(saved.status, "confirmed");
}

#[sqlx::test]
async fn get_returns_400_if_confirmation_token_is_absent(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let response = server.get_subscriptions_confirm().await;
    assert_eq!(response.status().as_u16(), 400);
}
