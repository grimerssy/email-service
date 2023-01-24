use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::DbPool;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(params, pool))]
pub async fn confirm_subscription(params: Query<Parameters>, pool: Data<DbPool>) -> HttpResponse {
    let subscriber_id = match get_subscriber_id_from_token(&params.subscription_token, &pool).await
    {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscriber_id = match subscriber_id {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().finish(),
    };
    confirm_subscriber(&subscriber_id, &pool)
        .await
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[tracing::instrument(
    name = "Getting subscriber id from token",
    skip(subscription_token, pool)
)]
async fn get_subscriber_id_from_token(
    subscription_token: &str,
    pool: &DbPool,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query!(
        r#"
        select subscriber_id from subscription_tokens
        where subscription_token = $1;
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map(|r| r.map(|o| o.subscriber_id))
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
}

#[tracing::instrument(name = "Confirming a pending subscriber", skip(subscriber_id, pool))]
async fn confirm_subscriber(subscriber_id: &Uuid, pool: &DbPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        update subscriptions
        set status = 'confirmed'
        where id = $1;
        "#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
}
