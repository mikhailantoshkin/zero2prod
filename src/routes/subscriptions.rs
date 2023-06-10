use axum::{
    extract::Form,
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Subscriber {
    name: String,
    email: String,
}

pub async fn subscriptions(Form(subscriber): Form<Subscriber>) -> StatusCode {
    StatusCode::OK
}
