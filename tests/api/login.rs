use crate::{TestServer, TestUser};
use hashmap_macro::hashmap;
use zero2prod::DbPool;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let body = hashmap!(
        "username" => "random-username",
        "password" => "random-password"
    );
    let response = server.post_login(&body).await;
    server.assert_is_redirect_to(&response, "/login");

    let html_page = server.get_login().await.text().await.unwrap();
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = server.get_login().await.text().await.unwrap();
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[sqlx::test]
async fn redirect_to_admin_dashboard_on_success(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::stored(&server.db_pool).await;
    let response = user.login(&server).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = server.get_admin_dashboard().await.text().await.unwrap();
    assert!(html_page.contains(&format!("Welcome, {}", user.username)));
}
