use actix_web::{
    http::header::LOCATION,
    web::{Data, Form},
    HttpResponse, ResponseError,
};
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

impl ResponseError for LoginError {
    fn status_code(&self) -> reqwest::StatusCode {
        reqwest::StatusCode::SEE_OTHER
    }
    fn error_response(&self) -> HttpResponse {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={encoded_error}")))
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip_all,
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(form: Form<FormData>, pool: Data<DbPool>) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::Auth(e.into()),
            AuthError::Unexpected(_) => LoginError::Unexpected(e.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}
