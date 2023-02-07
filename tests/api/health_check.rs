use crate::TestServer;
use zero2prod::DbPool;

#[sqlx::test]
async fn get_returns_200(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let response = server.get_health_check().await;
    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
