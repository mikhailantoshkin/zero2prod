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

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();
    let other_new_pass = Uuid::new_v4().to_string();

    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    let response = app
        .post_change_password(&serde_json::json!({
                "current_password": &app.test_user.password,
                "new_password": &new_pass,
                "new_password_check": &other_new_pass,
        }))
        .await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();
    let wron_pass = Uuid::new_v4().to_string();

    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    let response = app
        .post_change_password(&serde_json::json!({
                "current_password": &wron_pass,
                "new_password": &new_pass,
                "new_password_check": &new_pass,
        }))
        .await;
    assert_is_redirected_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn change_password_works() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();

    let response = app
        .login(&app.test_user.username, &app.test_user.password)
        .await;

    assert_is_redirected_to(&response, "/admin/dashboard");

    let response = app
        .post_change_password(&serde_json::json!({
                "current_password": &app.test_user.password,
                "new_password": &new_pass,
                "new_password_check": &new_pass,
        }))
        .await;
    assert_is_redirected_to(&response, "/admin/password");

    let response = app.get_change_password().await;
    // TODO: don't invalidate session on password change
    // let html_page = app.get_change_password_html().await;
    // assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"), "html {}", html_page);

    // let response = app.post_logout().await;
    assert_is_redirected_to(&response, "/login");

    // let html_page = app.get_login_html().await;
    // assert!(html_page.contains("<p><i>You have successuflly logged out.</i></p>"));

    let response = app.login(&app.test_user.username, &new_pass).await;
    assert_is_redirected_to(&response, "/admin/dashboard");
}
