mod confirm;
use anyhow::Context;
pub use confirm::*;
use sqlx::Transaction;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    server::AppBaseUrl,
    Database, DbPool, EmailClient,
};
use actix_web::{
    web::{Data, Form},
    HttpResponse, ResponseError,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    Validation(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            Self::Validation(_) => reqwest::StatusCode::BAD_REQUEST,
            Self::Unexpected(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryInto<NewSubscriber> for FormData {
    type Error = String;

    fn try_into(self) -> Result<NewSubscriber, Self::Error> {
        let name = SubscriberName::try_from(self.name)?;
        let email = SubscriberEmail::try_from(self.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email,
    )
)]
pub async fn subscribe(
    Form(form): Form<FormData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
    base_url: Data<AppBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let subscriber = form.try_into().map_err(SubscribeError::Validation)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire connection from the pool")?;
    let subscriber_id = insert_subscriber(&mut transaction, &subscriber).await?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, &subscriber_id, &subscription_token).await?;
    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;
    send_confirmation_email(
        &email_client,
        &subscriber,
        base_url.as_ref().as_ref(),
        &subscription_token,
    )
    .await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, transaction)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Database>,
    subscriber: &NewSubscriber,
) -> anyhow::Result<Uuid> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        insert into subscriptions (id, name, email, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending_confirmation');
        "#,
        subscriber_id,
        subscriber.name.as_ref(),
        subscriber.email.as_ref(),
        Utc::now(),
    )
    .execute(transaction)
    .await?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(subscriber_id, subscription_token, transaction)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Database>,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        insert into subscription_tokens (subscription_token, subscriber_id)
        values ($1, $2);
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(transaction)
    .await
    .map(|_| ())
    .map_err(anyhow::Error::from)
}

#[tracing::instrument(
    name = "Sending a confirmation email to a new subscriber",
    skip(email_client, subscriber)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> anyhow::Result<()> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let subject = "Subject";
    let text_body = &format!(
        r#"
        Welcome to the newsletter.
        Visit {} to confirm you subscription.
        "#,
        confirmation_link
    );
    let html_body = &format!(
        r#"
        Welcome to the newsletter.
        <br>
        Click <a href="{}">here</a> to confirm your subscription.
        "#,
        confirmation_link
    );
    email_client
        .send_email(&subscriber.email, subject, text_body, html_body)
        .await
        .map_err(anyhow::Error::from)
}

fn generate_subscription_token() -> String {
    use rand::{distributions::Alphanumeric, thread_rng, Rng};
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
