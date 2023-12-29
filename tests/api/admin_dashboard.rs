use crate::helpers::{assert_is_redirected_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;
    let response = app
        .login(&app.test_user.username, &app.test_user.password)
        .await;
    assert_is_redirected_to(&response, "/admin/dashboard");

    let html = app.get_admin_dashboard_html().await;
    assert!(html.contains(&format!("Welcome {}", app.test_user.username)));

    let response = app.post_logout().await;
    assert_is_redirected_to(&response, "/login");

    let html = app.get_login_html().await;
    assert!(html.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    let response = app.get_admin_dashboard().await;
    assert_is_redirected_to(&response, "/login")
}
