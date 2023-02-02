use crate::{utils::see_other, Session};
use actix_web::{
    error::InternalError,
    web::{Data, Form},
    HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use secrecy::Secret;
use serde::Deserialize;

use crate::{
    auth::{validate_credentials, AuthError, Credentials},
    DbPool,
};

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    Auth(#[source] anyhow::Error),
    #[error("Something went wrong")]
    Unexpected(#[from] anyhow::Error),
}

#[derive(Debug, Deserialize, Clone)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip_all,
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    session: Session,
    form: Form<FormData>,
    pool: Data<DbPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    let user_id =
        validate_credentials(credentials, &pool)
            .await
            .map_err(|e| {
                let e = match e {
                    AuthError::InvalidCredentials(_) => {
                        LoginError::Auth(e.into())
                    }
                    AuthError::Unexpected(_) => {
                        LoginError::Unexpected(e.into())
                    }
                };
                login_redirect(e)
            })?;
    session.renew();
    session
        .insert_user_id(user_id)
        .context("Failed to persist user session")
        .map_err(|e| login_redirect(e.into()))?;
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
    Ok(see_other("/admin/dashboard"))
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    InternalError::from_response(e, see_other("/login"))
}
