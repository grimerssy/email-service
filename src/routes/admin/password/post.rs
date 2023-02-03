use crate::{
    auth::{self, validate_credentials, AuthError, Credentials, UserId},
    routes::get_username,
    utils::{e500, see_other},
    DbPool,
};

use actix_web::{
    web::{Data, Form, ReqData},
    HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    user_id: ReqData<UserId>,
    form: Form<FormData>,
    pool: Data<DbPool>,
) -> actix_web::Result<HttpResponse> {
    let user_id = *user_id.into_inner();
    if form.new_password.expose_secret()
        != form.new_password_check.expose_secret()
    {
        FlashMessage::error(
            "You entered two different new passwords - \
             the field values must match.",
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
                FlashMessage::error("The current password is incorrect.")
                    .send();
                Ok(see_other("/admin/password"))
            }
            AuthError::Unexpected(_) => Err(e500(e)),
        };
    }
    auth::change_password(&user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::info("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
