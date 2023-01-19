use crate::TestServer;

#[tokio::test]
async fn returns_200() {
    let server = TestServer::run().await;
    let response = server.get_health_check().await;
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
