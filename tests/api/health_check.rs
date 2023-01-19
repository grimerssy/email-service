use crate::Server;

#[tokio::test]
async fn returns_200() {
    let server = Server::init().await;

    let url = format!("{}/health_check", server.config.application.addr());
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
