mod confirm;
pub use confirm::*;
use sqlx::Transaction;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    server::AppBaseUrl,
    Database, DbPool, EmailClient,
};
use actix_web::{
    web::{Data, Form},
    HttpResponse,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

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
) -> HttpResponse {
    let subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscriber_id = match insert_subscriber(&mut transaction, &subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, &subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    send_confirmation_email(
        &email_client,
        &subscriber,
        base_url.as_ref().as_ref(),
        &subscription_token,
    )
    .await
    .map(|_| HttpResponse::Ok().finish())
    .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, transaction)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Database>,
    subscriber: &NewSubscriber,
) -> sqlx::Result<Uuid> {
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
    .await
    .map(|_| subscriber_id)
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(subscriber_id, subscription_token, transaction)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Database>,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> sqlx::Result<()> {
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
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
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
) -> reqwest::Result<()> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    email_client
        .send_email(
            &subscriber.email,
            "Subject",
            &format!(
                r#"
        Welcome to the newsletter.
        <br>
        Click <a href="{}">here</a> to confirm your subscription.
        "#,
                confirmation_link
            ),
            &format!(
                r#"
        Welcome to the newsletter.
        Visit {} to confirm you subscription.
        "#,
                confirmation_link
            ),
        )
        .await
}

fn generate_subscription_token() -> String {
    use rand::{distributions::Alphanumeric, thread_rng, Rng};
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
