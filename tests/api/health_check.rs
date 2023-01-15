use crate::Server;

#[tokio::test]
async fn it_works() {
    let server = Server::spawn().await;

    let url = format!("{}/health_check", server.address);
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
