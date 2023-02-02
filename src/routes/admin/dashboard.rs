use crate::{
    utils::{e500, see_other},
    Session,
};
use actix_web::{http::header::ContentType, web::Data, HttpResponse};
use anyhow::Context;
use uuid::Uuid;

use crate::DbPool;

pub async fn admin_dashboard(
    session: Session,
    pool: Data<DbPool>,
) -> actix_web::Result<HttpResponse> {
    let username = match session.get_user_id()? {
        Some(user_id) => get_username(user_id, &pool).await.map_err(e500)?,
        None => return Ok(see_other("/login")),
    };
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
