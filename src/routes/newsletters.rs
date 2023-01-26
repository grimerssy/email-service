use actix_web::{
    web::{Data, Json},
    HttpResponse, ResponseError,
};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use tracing::warn;

use crate::{domain::SubscriberEmail, DbPool, EmailClient};

#[derive(Clone, Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for PublishError {}

#[tracing::instrument(name = "Publishing a newsletter", skip(body, pool, email_client), fields(email_subject = %body.title))]
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        let email = match subscriber {
            Ok(subscriber) => subscriber.email,
            Err(err) => {
                warn!(
                    err.cause_chain = ?err,
                    "Skipping a confirmed subscriber. \
                     Their stored contact details are invalid",
                );
                continue;
            }
        };
        email_client
            .send_email(&email, &body.title, &body.content.text, &body.content.html)
            .await
            .with_context(|| format!("Failed to send newsletter issue to {}", &email))?;
    }
    Ok(HttpResponse::Ok().finish())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip_all)]
async fn get_confirmed_subscribers(
    pool: &DbPool,
) -> anyhow::Result<Vec<anyhow::Result<ConfirmedSubscriber>>> {
    sqlx::query!(
        r#"
        select email from subscriptions
        where status = 'confirmed';
        "#
    )
    .fetch_all(pool)
    .await
    .map(|r| {
        r.into_iter()
            .map(|r| {
                SubscriberEmail::try_from(r.email)
                    .map(|email| ConfirmedSubscriber { email })
                    .map_err(|e| anyhow!(e))
            })
            .collect()
    })
    .map_err(anyhow::Error::from)
}
