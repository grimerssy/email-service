use super::{SubscriberEmail, SubscriberName};

#[derive(Clone, Debug)]
pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}
