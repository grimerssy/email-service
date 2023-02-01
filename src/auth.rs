use crate::{telemetry, DbPool};
use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
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
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &DbPool,
) -> Result<Uuid, AuthError> {
    let (user_id, expected_password_hash) =
        get_stored_credentials(&credentials.username, pool)
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
    .context("Failed to spawn blocking task")??;
    user_id
        .ok_or_else(|| anyhow!("Unknown username"))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Verifying password hash", skip_all)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash =
        PasswordHash::new(expected_password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")?;
    Argon2::default()
        .verify_password(
            password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}
