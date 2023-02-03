use hashmap_macro::hashmap;
use test_server::TestServer;
use uuid::Uuid;

use crate::{ServerExt, TestUser};

#[macros::test]
async fn change_password_works(server: TestServer) {
    let mut user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    tracing::error!("{body:?}");
    let response = server.post_login(&body).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();
    let body = hashmap!(
        "current_password" => user.password.as_str(),
        "new_password" => new_password.as_str(),
        "new_password_check" => new_password.as_str(),
    );
    let response = server.post_admin_password(&body).await;
    server.assert_is_redirect_to(&response, "/admin/password");

    let html_page = server.get_admin_password().await.text().await.unwrap();
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = server.post_admin_logout().await;
    server.assert_is_redirect_to(&response, "/login");

    let html_page = server.get_login().await.text().await.unwrap();
    assert!(
        html_page.contains("<p><i>You have successfully logged out.</i></p>")
    );

    user.password = new_password;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    let response = server.post_login(&body).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");
}

#[macros::test]
async fn unauthenticated_users_can_not_access_form(server: TestServer) {
    let response = server.get_admin_password().await;
    server.assert_is_redirect_to(&response, "/login");
}

#[macros::test]
async fn unauthenticated_users_can_not_change_password(server: TestServer) {
    let password = Uuid::new_v4().to_string();
    let body = hashmap!(
        "current_password" => password.as_str(),
        "new_password" => password.as_str(),
        "new_password_check" => password.as_str(),
    );
    let response = server.post_admin_password(&body).await;
    server.assert_is_redirect_to(&response, "/login");
}

#[macros::test]
async fn current_password_must_be_valid(server: TestServer) {
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;

    let wrong_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();
    let body = hashmap!(
        "current_password" => wrong_password.as_str(),
        "new_password" => new_password.as_str(),
        "new_password_check" => new_password.as_str(),
    );
    let response = server.post_admin_password(&body).await;
    server.assert_is_redirect_to(&response, "/admin/password");

    let html_page = server.get_admin_password().await.text().await.unwrap();
    assert!(
        html_page.contains("<p><i>The current password is incorrect.</i></p>")
    )
}

#[macros::test]
async fn new_password_fields_must_match(server: TestServer) {
    let user = TestUser::stored(&server.db_pool).await;
    let body = hashmap!(
        "username" => user.username.as_str(),
        "password" => user.password.as_str(),
    );
    server.post_login(&body).await;

    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();
    let body = hashmap!(
        "current_password" => user.password.as_str(),
        "new_password" => new_password.as_str(),
        "new_password_check" => another_new_password.as_str(),
    );
    let response = server.post_admin_password(&body).await;
    server.assert_is_redirect_to(&response, "/admin/password");

    let html_page = server.get_admin_password().await.text().await.unwrap();
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
         the field values must match.</i></p>"
    ))
}
