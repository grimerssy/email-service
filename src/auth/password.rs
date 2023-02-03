use crate::{
    telemetry::{self, spawn_blocking_with_tracing},
    DbPool,
};
use anyhow::{anyhow, Context};
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash,
    PasswordHasher, PasswordVerifier, Version,
};
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

#[tracing::instrument(name = "Change password", skip_all)]
pub async fn change_password(
    user_id: &Uuid,
    password: Secret<String>,
    pool: &DbPool,
) -> anyhow::Result<()> {
    let password_hash =
        spawn_blocking_with_tracing(move || compute_password_hash(password))
            .await?
            .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        update users
        set password_hash = $1
        where user_id = $2;
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .map(|_| ())
    .context("Failed to change user's password in the database")
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

fn compute_password_hash(
    password: Secret<String>,
) -> anyhow::Result<Secret<String>> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)
    .map(|h| h.to_string())
    .map(Secret::new)
    .map_err(anyhow::Error::from)
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

#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}
