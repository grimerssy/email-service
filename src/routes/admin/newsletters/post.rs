use crate::{
    auth::UserId,
    idempotency::{store_response, try_process, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
    Database, DbPool,
};
use actix_web::{
    web::{Data, Form, ReqData},
    HttpResponse, ResponseError,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::Deserialize;
use sqlx::Transaction;
use uuid::Uuid;

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
) -> actix_web::Result<HttpResponse> {
    let user_id = *user_id.into_inner();
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey =
        idempotency_key.try_into().map_err(e400)?;
    let asdf = try_process(&user_id, &idempotency_key, &pool)
        .await
        .map_err(e500)?;
    let mut transaction = match asdf {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            send_success_message();
            return Ok(response);
        }
    };
    let issue_id = insert_newsletter_issues(
        &title,
        &text_content,
        &html_content,
        &mut transaction,
    )
    .await
    .context("Failed to store newsletter issue details")
    .map_err(e500)?;
    enqueue_delivery_tasks(&issue_id, &mut transaction)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;
    let response = see_other("/admin/newsletters");
    let response =
        store_response(&user_id, &idempotency_key, response, transaction)
            .await
            .map_err(e500)?;
    send_success_message();
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issues(
    title: &str,
    text_content: &str,
    html_content: &str,
    transaction: &mut Transaction<'_, Database>,
) -> sqlx::Result<Uuid> {
    let issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        insert into newsletter_issues (
           newsletter_issue_id,
           title,
           text_content,
           html_content,
           published_at
        )
        values ($1, $2, $3, $4, now());
        "#,
        issue_id,
        title,
        text_content,
        html_content,
    )
    .execute(transaction)
    .await
    .map(|_| issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    newsletter_issue_id: &Uuid,
    transaction: &mut Transaction<'_, Database>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        insert into issue_delivery_queue(
           newsletter_issue_id,
           subscriber_email
        )
        select $1, email
        from subscriptions
        where status = 'confirmed';
        "#,
        newsletter_issue_id
    )
    .execute(transaction)
    .await
    .map(|_| ())
}
