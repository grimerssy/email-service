use crate::{
    auth::UserId,
    domain::SubscriberEmail,
    idempotency::{get_saved_response, store_response, IdempotencyKey},
    utils::{e400, e500, see_other},
    DbPool, EmailClient,
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
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
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

fn send_success_message() {
    FlashMessage::info("You have successfully published a newsletter.").send();
}

#[tracing::instrument(
    name = "Publishing a newsletter",
    skip_all,
    fields(email_subject = %form.title)
)]
pub async fn publish_newsletter(
    user_id: ReqData<UserId>,
    form: Form<FormData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
) -> actix_web::Result<HttpResponse> {
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey =
        idempotency_key.try_into().map_err(e400)?;
    if let Some(response) =
        get_saved_response(&user_id, &idempotency_key, &pool)
            .await
            .map_err(e500)?
    {
        send_success_message();
        return Ok(response);
    }
    let user_id = *user_id.into_inner();
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
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
            .send_email(&email, &title, &text_content, &html_content)
            .await
            .with_context(|| {
                format!("Failed to send newsletter issue to {}", &email)
            })
            .map_err(e500)?;
    }
    send_success_message();
    let response = see_other("/admin/newsletters");
    let response = store_response(&user_id, &idempotency_key, response, &pool)
        .await
        .map_err(e500)?;
    Ok(response)
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
