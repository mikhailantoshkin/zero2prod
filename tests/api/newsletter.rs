use std::collections::HashMap;

use crate::helpers::{assert_is_redirected_to, spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscription(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let conformation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(conformation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_uncofirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let key = uuid::Uuid::new_v4().to_string();
    let newsletter_request_body = HashMap::from([
        ("title", "Newsletter title"),
        ("text", "Newsletter body as plain text"),
        ("html", "<p>Newsletter body as HTML</p>"),
        ("idempotency_key", key.as_str()),
    ]);

    let response = app.post_publish_newsletters(&newsletter_request_body).await;

    assert_is_redirected_to(&response, "/admin/newsletter")
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let key = uuid::Uuid::new_v4().to_string();
    let newsletter_request_body = HashMap::from([
        ("title", "Newsletter title"),
        ("text", "Newsletter body as plain text"),
        ("html", "<p>Newsletter body as HTML</p>"),
        ("idempotency_key", key.as_str()),
    ]);

    let response = app.post_publish_newsletters(&newsletter_request_body).await;

    assert_is_redirected_to(&response, "/admin/newsletter")
}

#[tokio::test]
async fn newsletters_return_400_for_invalid_data() {
    let app = spawn_app().await;
    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    let test_cases = vec![
        (
            HashMap::from([
                ("text", "Newsletter body as plain text"),
                ("html", "<p>Newsletter body as HTML</p>"),
            ]),
            "missing title",
        ),
        (
            HashMap::from([("title", "Newsletter title")]),
            "missing content",
        ),
    ];

    for (invalid_body, error_msg) in test_cases {
        let response = app.post_publish_newsletters(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            422,
            "Api did not fail with 400 when payload was {}.",
            error_msg
        );
    }
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;
    let key = uuid::Uuid::new_v4().to_string();
    let body = HashMap::from([
        ("title", "Newsletter title"),
        ("text", "Newsletter body as plain text"),
        ("html", "<p>Newsletter body as HTML</p>"),
        ("idempotency_key", key.as_str()),
    ]);
    let response = app.post_publish_newsletters(&body).await;
    assert_is_redirected_to(&response, "/login")
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let key = uuid::Uuid::new_v4().to_string();
    let body = HashMap::from([
        ("title", "Newsletter title"),
        ("text", "Newsletter body as plain text"),
        ("html", "<p>Newsletter body as HTML</p>"),
        ("idempotency_key", key.as_str()),
    ]);
    let response = app.post_publish_newsletters(&body).await;
    assert_is_redirected_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    let response = app.post_publish_newsletters(&body).await;
    assert_is_redirected_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}
