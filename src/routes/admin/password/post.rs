use actix_web::{web, HttpResponse};
use secrecy::Secret;
use serde::Deserialize;

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
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }
    todo!()
}
