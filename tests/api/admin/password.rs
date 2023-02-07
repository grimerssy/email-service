use crate::{TestServer, TestUser};
use hashmap_macro::hashmap;
use uuid::Uuid;
use zero2prod::DbPool;

#[sqlx::test]
async fn change_password_works(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let mut user = TestUser::stored(&server.db_pool).await;
    let response = user.login(&server).await;
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
    let response = user.login(&server).await;
    server.assert_is_redirect_to(&response, "/admin/dashboard");
}

#[sqlx::test]
async fn unauthenticated_users_can_not_access_form(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let response = server.get_admin_password().await;
    server.assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn unauthenticated_users_can_not_change_password(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let password = Uuid::new_v4().to_string();
    let body = hashmap!(
        "current_password" => password.as_str(),
        "new_password" => password.as_str(),
        "new_password_check" => password.as_str(),
    );
    let response = server.post_admin_password(&body).await;
    server.assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn current_password_must_be_valid(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;

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

#[sqlx::test]
async fn new_password_fields_must_match(pool: DbPool) {
    let server = TestServer::run(pool).await;
    let user = TestUser::stored(&server.db_pool).await;
    user.login(&server).await;

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
