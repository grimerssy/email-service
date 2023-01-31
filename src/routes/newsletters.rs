use actix_web::{
    http::header::{HeaderMap, HeaderValue},
    web::{Data, Json},
    HttpRequest, HttpResponse, ResponseError,
};
use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use reqwest::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use tracing::warn;
use uuid::Uuid;

use crate::{domain::SubscriberEmail, telemetry, DbPool, EmailClient};

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
    #[error("Authentication failed")]
    Auth(#[source] anyhow::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            Self::Auth(_) => reqwest::StatusCode::UNAUTHORIZED,
            Self::Unexpected(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            Self::Auth(_) => {
                let mut response = HttpResponse::new(self.status_code());
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(WWW_AUTHENTICATE, header_value);
                response
            }
            _ => HttpResponse::new(self.status_code()),
        }
    }
}

#[tracing::instrument(name = "Publishing a newsletter", skip(body, pool, email_client), fields(email_subject = %body.title))]
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: Data<DbPool>,
    email_client: Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let creds = basic_auth(request.headers())
        .await
        .map_err(PublishError::Auth)?;
    let user_id = validate_credentials(creds, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
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

#[tracing::instrument(name = "Getting stored credentials", skip_all)]
async fn get_stored_credentials(
    username: &str,
    pool: &DbPool,
) -> anyhow::Result<Option<(Uuid, Secret<String>)>> {
    let row = sqlx::query!(
        r#"
        select user_id, password_hash from users
        where username = $1;
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|r| (r.user_id, Secret::new(r.password_hash)));
    Ok(row)
}

#[tracing::instrument(name = "Validating user credentials", skip_all)]
async fn validate_credentials(
    credentials: Credentials,
    pool: &DbPool,
) -> Result<Uuid, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials.username, pool)
        .await?
        .map(|(user_id, eph)| (Some(user_id), eph))
        .unwrap_or_else(|| {
            (
                None,
                Secret::new(
                    "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                        .to_string(),
                ),
            )
        });
    telemetry::spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")?
    .map_err(PublishError::Auth)?;
    user_id
        .ok_or_else(|| anyhow!("Unknown username"))
        .map_err(PublishError::Auth)
}

#[tracing::instrument(name = "Verifying password hash", skip_all)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password: Secret<String>,
) -> anyhow::Result<()> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid password.")
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(name = "Extracting auth credentials from header", skip_all)]
async fn basic_auth(headers: &HeaderMap) -> anyhow::Result<Credentials> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .with_context(|| format!("The '{AUTHORIZATION}' header was missing"))?
        .to_str()
        .with_context(|| format!("The '{AUTHORIZATION}' header was not a valid UTF8 string"))?
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;
    let decoded = base64::decode_config(auth_header, base64::STANDARD)
        .map(String::from_utf8)
        .context("Failed to decode 'Basic' credentials")?
        .context("The decoded credentials string was not a valid UTF8")?;
    let mut creds = decoded.splitn(2, ':');
    let username = creds
        .next()
        .ok_or_else(|| anyhow!("A username must be provided in 'Basic' auth."))
        .map(String::from)?;
    let password = creds
        .next()
        .ok_or_else(|| anyhow!("A password must be provided in 'Basic' auth."))
        .map(String::from)
        .map(Secret::new)?;
    Ok(Credentials { username, password })
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
