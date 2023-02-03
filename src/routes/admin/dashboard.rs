use crate::{auth::UserId, utils::e500, DbPool};
use actix_web::{
    http::header::ContentType,
    web::{Data, ReqData},
    HttpResponse,
};
use anyhow::Context;
use uuid::Uuid;

pub async fn admin_dashboard(
    user_id: ReqData<UserId>,
    pool: Data<DbPool>,
) -> actix_web::Result<HttpResponse> {
    let user_id = *user_id.into_inner();
    let username = get_username(user_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
            </head>
            <body>
                <p>Welcome, {username}!</p>
                <p>Available actions:</p>
                <ol>
                    <li><a href="/admin/newsletters">Post a newsletter</a></li>
                    <li><a href="/admin/password">Change password</a></li>
                    <li>
                        <form name="logoutForm" action="/admin/logout" method="post">
                            <input type="submit" value="Logout">
                        </form>
                    </li>
                </ol>
            </body>
            </html>
            "#
        )))
}

#[tracing::instrument(name = "Get username", skip_all)]
pub async fn get_username(
    user_id: Uuid,
    pool: &DbPool,
) -> anyhow::Result<String> {
    sqlx::query!(
        r#"
        select username
        from users
        where user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to query for a username")
    .map(|r| r.username)
}
