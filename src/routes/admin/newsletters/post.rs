use std::str::FromStr;

use crate::{authentication::UserId, utils::see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;

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
    html_content: String,
    text_content: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(params, pool, email_client, user_id),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    pool: web::Data<sqlx::PgPool>,
    email_client: web::Data<EmailClient>,
    params: web::Form<PublishParams>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PublishError> {
    tracing::Span::current().record("user_id", tracing::field::display(*user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for s in subscribers {
        match s {
            Ok(s) => {
                email_client
                    .send_email(
                        &s.email,
                        &params.title,
                        &params.html_content,
                        &params.text_content,
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
    FlashMessage::info("The newsletter issue has been published!").send();
    Ok(see_other("/admin/newsletters"))
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
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
