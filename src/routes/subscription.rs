use std::str::FromStr;

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormSubscribe {
    name: String,
    email: String,
}

impl TryFrom<FormSubscribe> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormSubscribe) -> Result<Self, Self::Error> {
        let name = SubscriberName::from_str(&value.name)?;
        let email = value.email;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "add a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(form: web::Form<FormSubscribe>, pool: web::Data<PgPool>) -> impl Responder {
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_subscriber(pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Save new subscriber to db", skip(new_subscriber, pool))]
pub async fn insert_subscriber(
    pool: web::Data<PgPool>,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    // We use `get_ref` to get an immutable reference to the `PgConnection`
    // wrapped by `web::Data`.
    .execute(pool.get_ref())
    .await
    .map_err(|e| {
        tracing::error!("Error inserting subscriber: {:?}", e);
        e
    })?;
    Ok(())
}
