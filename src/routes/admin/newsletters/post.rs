use crate::{
    auth::UserId, domain::SubscriberEmail, utils::see_other, DbPool,
    EmailClient,
};
use actix_web::{
    web::{Data, Form, ReqData},
    HttpResponse, ResponseError,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::{anyhow, Context};
use serde::Deserialize;
use tracing::warn;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyData {
    title: String,
    text_content: String,
    html_content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[tracing::instrument(name = "Publishing a newsletter", skip(body, pool, email_client), fields(email_subject = %body.title))]
pub async fn publish_newsletter(
    user_id: ReqData<UserId>,
    body: Form<BodyData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let user_id = *user_id.into_inner();
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
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
            .send_email(
                &email,
                &body.title,
                &body.text_content,
                &body.html_content,
            )
            .await
            .with_context(|| {
                format!("Failed to send newsletter issue to {}", &email)
            })?;
    }
    FlashMessage::info("You have successfully published a newsletter.").send();
    Ok(see_other("/admin/newsletters"))
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

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
