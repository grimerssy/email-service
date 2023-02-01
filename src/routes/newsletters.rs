use crate::{
    auth::{validate_credentials, AuthError, Credentials},
    domain::SubscriberEmail,
    DbPool, EmailClient,
};
use actix_web::{
    http::header::{HeaderMap, HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE},
    web::{Data, Json},
    HttpRequest, HttpResponse, ResponseError,
};
use anyhow::{anyhow, Context};
use secrecy::Secret;
use serde::Deserialize;
use tracing::warn;

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
    let user_id = validate_credentials(creds, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::Auth(e.into()),
            AuthError::Unexpected(_) => PublishError::Unexpected(e.into()),
        })?;
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
