use hashmap_macro::hashmap;
use test_server::TestServer;

use crate::{ServerExt, TestUser};

#[macros::test]
async fn unauthenticated_users_are_redirected_to_login(server: TestServer) {
    let response = server.get_admin_dashboard().await;
    server.assert_is_redirect_to(&response, "/login");
}

#[macros::test]
async fn logout_clears_session_state(server: TestServer) {
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    let response = server.post_login(&body).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = server.get_admin_dashboard().await.text().await.unwrap();
    assert!(html_page.contains(&format!("Welcome, {}", &user.username)));

    let response = server.post_logout().await;
    server.assert_is_redirect_to(&response, "/login");

    let html_page = server.get_login().await.text().await.unwrap();
    assert!(html_page
        .contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    let response = server.get_admin_dashboard().await;
    server.assert_is_redirect_to(&response, "/login");
}
