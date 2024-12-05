use std::str::FromStr;

use crate::{
    authentication::UserId,
    idempotency::{get_saved_response, save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;

use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(serde::Deserialize)]
pub struct PublishParams {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
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
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let PublishParams {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = params.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let tx = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartingProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for s in subscribers {
        match s {
            Ok(s) => {
                email_client
                    .send_email(&s.email, &title, &html_content, &text_content)
                    .await
                    .with_context(|| format!("failed to send newsletter to {:?}", s.email))
                    .map_err(e500)?;
            }
            Err(e) => {
                tracing::warn!(
                    error.cause_chain = ?e,
                    "skip for invalid email for {}", e);
            }
        }
    }
    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(tx, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
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
