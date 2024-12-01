use anyhow::Context;
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument("Validate credentials", skip(pool, credentials))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw\
        $CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&pool, &credentials.username).await?
    {
        user_id = Some(stored_user_id);
        expected_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || verify_password_hash(expected_hash, credentials.password))
        .await
        .context("failed to spawn password verification task")??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Invalid credentials")))
}

#[tracing::instrument("Verify password hash", skip(expected_hash, password))]
pub fn verify_password_hash(
    expected_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(&expected_hash.expose_secret())
        .context("failed to parse hash in PHC string format")?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)?;

    Ok(())
}

#[tracing::instrument("Get stored credentials", skip(pool, username))]
pub async fn get_stored_credentials(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        "select user_id, password_hash from users where username = $1",
        username
    )
    .fetch_optional(pool)
    .await
    .context("failed to query user credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument("Change password", skip(pool, password))]
pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("failed to hash password")?;

    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE user_id = $2",
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("failed to update password in db")?;

    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}
