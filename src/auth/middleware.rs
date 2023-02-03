use std::ops::Deref;

use crate::{utils::see_other, Session};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::InternalError,
    FromRequest, HttpMessage,
};
use actix_web_lab::middleware::Next;
use anyhow::anyhow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonynous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> actix_web::Result<ServiceResponse<impl MessageBody>> {
    let session = {
        let (http_requst, payload) = req.parts_mut();
        Session::from_request(http_requst, payload).await
    }?;
    match session.get_user_id()? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            next.call(req).await
        }
        None => {
            let e = anyhow!("User is not logged in");
            Err(InternalError::from_response(e, see_other("/login")).into())
        }
    }
}
