use actix_web::{
    web::{Data, Form},
    HttpResponse,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email,
    )
)]
pub async fn subscribe(form: Form<FormData>, pool: Data<PgPool>) -> HttpResponse {
    insert_subscriber(&form, &pool)
        .await
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
async fn insert_subscriber(form: &FormData, pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        insert into subscriptions (id, name, email, subscribed_at)
        values ($1, $2, $3, $4);
    "#,
        Uuid::new_v4(),
        form.name,
        form.email,
        Utc::now(),
    )
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
}
