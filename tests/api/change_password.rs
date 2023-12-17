use crate::helpers::{assert_is_redirected_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn must_be_logged_in_to_see_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirected_to(&response, "/login")
}
