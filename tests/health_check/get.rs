use crate::{Server, TestServer};

#[macros::test]
async fn returns_200(server: TestServer) {
    let response = server.get_health_check().await;
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
