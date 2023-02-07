use crate::{TestServer, TestUser};
use zero2prod::DbPool;

#[sqlx::test]
async fn unauthenticated_users_are_redirected_to_login(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let response = server.get_admin_dashboard().await;
    server.assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn logout_clears_session_state(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::stored(&server.db_pool).await;
    let response = user.login(&server).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = server.get_admin_dashboard().await.text().await.unwrap();
    assert!(html_page.contains(&format!("Welcome, {}", &user.username)));

    let response = server.post_admin_logout().await;
    server.assert_is_redirect_to(&response, "/login");

    let html_page = server.get_login().await.text().await.unwrap();
    assert!(html_page
        .contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    let response = server.get_admin_dashboard().await;
    server.assert_is_redirect_to(&response, "/login");
}
