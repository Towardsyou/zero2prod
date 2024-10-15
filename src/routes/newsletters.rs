use std::str::FromStr;

use actix_web::{web, HttpResponse};
use anyhow::Context;

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {}

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
) -> Result<HttpResponse, PublishError> {
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
