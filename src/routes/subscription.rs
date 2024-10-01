use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::{types::Uuid, PgPool};
use tracing::Instrument;

#[derive(serde::Deserialize)]
pub struct FormSubscrib {
    name: String,
    email: String,
}

pub async fn subscribe(form: web::Form<FormSubscrib>, pool: web::Data<PgPool>) -> impl Responder {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
    "Adding a new subscriber.",
    %request_id,
    subscriber_email = %form.email,
    subscriber_name= %form.name
    );
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Saving new subscriber details in the database");
    match sqlx::query!(
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
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("new subscription inserted");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("error inserting new subscription {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
