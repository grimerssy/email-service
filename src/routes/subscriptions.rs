use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    DbPool, EmailClient,
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
    skip(form, pool, email_client),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email,
    )
)]
pub async fn subscribe(
    Form(form): Form<FormData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
) -> HttpResponse {
    let subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    if insert_subscriber(&subscriber, &pool).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };
    send_confirmation_email(&email_client, &subscriber)
        .await
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, pool)
)]
async fn insert_subscriber(subscriber: &NewSubscriber, pool: &DbPool) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        insert into subscriptions (id, name, email, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending_confirmation');
    "#,
        Uuid::new_v4(),
        subscriber.name.as_ref(),
        subscriber.email.as_ref(),
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

#[tracing::instrument(
    name = "Sending a confirmation email to a new subscriber",
    skip(email_client, subscriber)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
) -> reqwest::Result<()> {
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
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
