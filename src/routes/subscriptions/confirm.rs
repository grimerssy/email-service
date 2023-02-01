use actix_web::{
    web::{Data, Query},
    HttpResponse, ResponseError,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::DbPool;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmError {
    #[error("Failed to identify user")]
    UnknownUser,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for ConfirmError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            Self::UnknownUser => reqwest::StatusCode::UNAUTHORIZED,
            Self::Unexpected(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(params, pool)
)]
pub async fn confirm_subscription(
    params: Query<Parameters>,
    pool: Data<DbPool>,
) -> Result<HttpResponse, ConfirmError> {
    let subscriber_id =
        get_subscriber_id_from_token(&params.subscription_token, &pool)
            .await?
            .ok_or(ConfirmError::UnknownUser)?;
    confirm_subscriber(&subscriber_id, &pool)
        .await
        .map(|_| HttpResponse::Ok().finish())
        .map_err(ConfirmError::from)
}

#[tracing::instrument(
    name = "Getting subscriber id from token",
    skip(subscription_token, pool)
)]
async fn get_subscriber_id_from_token(
    subscription_token: &str,
    pool: &DbPool,
) -> anyhow::Result<Option<Uuid>> {
    sqlx::query!(
        r#"
        select subscriber_id from subscription_tokens
        where subscription_token = $1;
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map(|result| result.map(|record| record.subscriber_id))
    .map_err(anyhow::Error::from)
}

#[tracing::instrument(
    name = "Confirming a pending subscriber",
    skip(subscriber_id, pool)
)]
async fn confirm_subscriber(
    subscriber_id: &Uuid,
    pool: &DbPool,
) -> anyhow::Result<()> {
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
    .map_err(anyhow::Error::from)
}
