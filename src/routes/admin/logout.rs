use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::{utils::see_other, Session};

pub async fn logout(session: Session) -> actix_web::Result<HttpResponse> {
    if session.get_user_id()?.is_some() {
        session.logout();
        FlashMessage::info("You have successfully logged out.").send();
    }
    Ok(see_other("/login"))
}
