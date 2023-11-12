use crate::helpers::spawn_app;

pub fn assert_is_rederected_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "name",
        "password": "pass"
    });
    let response = app.post_login(&login_body).await;
    assert_is_rederected_to(&response, "/login")
}

#[tokio::test]
async fn an_error_flas_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "name",
        "password": "pass"
    });
    let response = app.post_login(&login_body).await;
    assert_is_rederected_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}
