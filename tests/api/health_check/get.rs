use sqlx::{Pool, Postgres};

use crate::TestServer;

#[sqlx::test]
async fn returns_200(pool: Pool<Postgres>) {
    let server = TestServer::run(pool).await;
    let response = server.get_health_check().await;
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
