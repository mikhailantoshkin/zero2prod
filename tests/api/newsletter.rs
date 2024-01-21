use std::collections::HashMap;
use std::time::Duration;

use crate::helpers::{assert_is_redirected_to, spawn_app, ConfirmationLinks, TestApp};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, MockBuilder, ResponseTemplate};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();

    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();
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

#[tokio::test]
async fn concurent_form_sumbission_is_handled_gracefully() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login(&app.test_user.username, &app.test_user.password)
        .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
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
    let response1 = app.post_publish_newsletters(&body);
    let response2 = app.post_publish_newsletters(&body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    )
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}

#[tokio::test]
async fn transiont_erros_do_not_cause_duplicat_deliveries_on_retries() {
    let app = spawn_app().await;
    let key = uuid::Uuid::new_v4().to_string();
    let body = HashMap::from([
        ("title", "Newsletter title"),
        ("text", "Newsletter body as plain text"),
        ("html", "<p>Newsletter body as HTML</p>"),
        ("idempotency_key", key.as_str()),
    ]);
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;

    app.login(&app.test_user.username, &app.test_user.password)
        .await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_publish_newsletters(&body).await;
    assert_eq!(response.status().as_u16(), 500);

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Delivery retry")
        .mount(&app.email_server)
        .await;

    let response = app.post_publish_newsletters(&body).await;
    assert_eq!(response.status().as_u16(), 303);
}
