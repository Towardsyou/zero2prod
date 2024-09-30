use actix_web::{web, HttpResponse, Responder};

#[derive(serde::Deserialize)]
pub struct FormSubscrib {
    name: String,
    email: String,
}

pub async fn subscribe(_form: web::Form<FormSubscrib>) -> impl Responder {
    HttpResponse::Ok()
}
