use actix_session::{Session as ActixSession, SessionExt};
use actix_web::FromRequest;
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct Session(ActixSession);

impl Session {
    const USER_ID_KEY: &'static str = "USER_ID";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn logout(&self) {
        self.0.purge();
    }

    pub fn insert_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<(), actix_session::SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(
        &self,
    ) -> Result<Option<Uuid>, actix_session::SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }
}

impl FromRequest for Session {
    type Error = <ActixSession as FromRequest>::Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Ok(Self(req.get_session())))
    }
}
