use actix_web::{
    error::InternalError,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse, ResponseError,
};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
};

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        StatusCode::SEE_OTHER
    }
}
#[derive(serde::Deserialize)]
pub struct LoginParams {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument("Login", skip(form, pool))]
pub async fn login(
    form: web::Form<LoginParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let cred = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&cred.username));
    match validate_credentials(cred, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            FlashMessage::error(e.to_string()).send();
            let resp = HttpResponse::SeeOther()
                .insert_header((LOCATION, format!("/login")))
                .finish();
            Err(InternalError::from_response(e, resp))
        }
    }
}
