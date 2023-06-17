use serde::Deserialize;

use super::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

#[derive(Deserialize)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
