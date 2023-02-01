use actix_web::{
    error::InternalError,
    http::header::LOCATION,
    web::{Data, Form},
    HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
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

#[derive(Debug, Clone, Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip_all,
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: Form<FormData>,
    pool: Data<DbPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    validate_credentials(credentials, &pool)
        .await
        .map(|user_id| {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));
            HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish()
        })
        .map_err(|e| {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::Auth(e.into()),
                AuthError::Unexpected(_) => LoginError::Unexpected(e.into()),
            };
            FlashMessage::error(e.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();
            InternalError::from_response(e, response)
        })
}
