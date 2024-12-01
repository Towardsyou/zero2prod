use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::admin::dashboard::get_username,
};

use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_confirmed: Secret<String>,
}

pub async fn change_password(
    session: TypedSession,
    form: web::Form<ChangePasswordForm>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    if form.new_password.expose_secret().len() < 12 {
        FlashMessage::error("Your new password must be at least 12 characters long.").send();
        return Ok(see_other("/admin/password"));
    }
    let user_id: uuid::Uuid = match session.get_user_id().map_err(e500)? {
        None => {
            return Ok(see_other("/login"));
        }
        Some(uid) => uid,
    };

    if form.new_password.expose_secret() != form.new_password_confirmed.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("Your current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
