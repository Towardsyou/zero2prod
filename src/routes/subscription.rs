use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormSubscribe {
    name: String,
    email: String,
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
    match insert_subscriber(form, pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Save new subscriber to db", skip(form, pool))]
pub async fn insert_subscriber(
    form: web::Form<FormSubscribe>,
    pool: web::Data<PgPool>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
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
