use crate::ServerExt;
use hashmap_macro::hashmap;
use test_server::TestServer;

#[macros::test]
async fn an_error_flash_message_is_set_on_failure(server: TestServer) {
    let body = hashmap!(
        "username" => "random-username",
        "password" => "random-password"
    );
    let response = server.post_login(&body).await;
    server.assert_is_redirect_to(&response, "/login");

    let html_page = server.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = server.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}
