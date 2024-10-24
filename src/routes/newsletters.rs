use std::str::FromStr;

use crate::authentication::{validate_credentials, AuthError, Credentials};
use actix_web::{http::header::HeaderMap, web, HttpRequest, HttpResponse};
use anyhow::Context;
use base64::Engine;
use secrecy::Secret;

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication error")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => HttpResponse::InternalServerError().finish(),
            PublishError::AuthError(_) => {
                let mut resp = HttpResponse::Unauthorized();
                resp.insert_header((
                    actix_web::http::header::WWW_AUTHENTICATE,
                    "Basic realm=\"publish\"",
                ));
                resp.finish()
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct PublishParams {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

pub async fn publish_newsletter(
    pool: web::Data<sqlx::PgPool>,
    email_client: web::Data<EmailClient>,
    params: web::Json<PublishParams>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(e) => PublishError::AuthError(e),
            AuthError::UnexpectedError(e) => PublishError::UnexpectedError(e),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for s in subscribers {
        match s {
            Ok(s) => {
                email_client
                    .send_email(
                        &s.email,
                        &params.title,
                        &params.content.html,
                        &params.content.text,
                    )
                    .await
                    .with_context(|| format!("failed to send newsletter to {:?}", s.email))?;
            }
            Err(e) => {
                tracing::warn!(
                    error.cause_chain = ?e,
                    "skip for invalid email for {}", e);
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument("Get confirmed subscriber", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &sqlx::PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let records = sqlx::query!("SELECT email FROM subscriptions where status='confirmed'",)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| match SubscriberEmail::from_str(&r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();
    Ok(records)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // The header value, if present, must be a valid UTF8 string
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_credentials = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_credentials = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_credentials)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_credentials)
        .context("The decoded credential string is valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
