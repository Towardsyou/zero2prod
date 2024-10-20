use actix_web::{
    error::InternalError,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse, ResponseError,
};
use hmac::Mac;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt, startup::HmacSecret,
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

#[tracing::instrument("Login", skip(form, pool, secret))]
pub async fn login(
    form: web::Form<LoginParams>,
    pool: web::Data<PgPool>,
    secret: web::Data<HmacSecret>,
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
            let error_msg = urlencoding::Encoded::new(e.to_string());
            let hmac_tag = {
                let mut mac =
                    hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
                        .unwrap();
                mac.update(error_msg.to_str().as_bytes());
                mac.finalize().into_bytes()
            };
            let resp = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    format!("/login?error={error_msg}&tag={hmac_tag:x}"),
                ))
                .finish();
            Err(InternalError::from_response(e, resp))
        }
    }
}
