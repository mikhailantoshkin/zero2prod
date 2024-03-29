use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn test_subscription_200() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let resp = test_app.post_subscription(body.into()).await;

    assert_eq!(200, resp.status().as_u16());
}

#[tokio::test]
async fn test_subscription_persists_subscriber() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscription(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.pool)
        .await
        .expect("Failed to fetch users");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_conformation");
}

#[tokio::test]
async fn test_422_on_bad_request() {
    let test_app = spawn_app().await;
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, reason) in cases.into_iter() {
        let resp = test_app.post_subscription(body.into()).await;
        assert_eq!(
            422,
            resp.status().as_u16(),
            "Expected status 400 because of {reason}",
        )
    }
}

#[tokio::test]
async fn test_422_on_bad_name() {
    let test_app = spawn_app().await;
    let cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, reason) in cases.into_iter() {
        let resp = test_app.post_subscription(body.into()).await;

        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap();
        assert_eq!(
            422, status,
            "Expected status 400 because of {reason}. Response {text}",
        )
    }
}

#[tokio::test]
async fn subscribe_sends_conformation_email_on_valid_request() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    // Act
    app.post_subscription(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    assert_eq!(links.html, links.plain_text)
}

#[tokio::test]
async fn subscribe_fails_if_there_is_fatal_db_error() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token",)
        .execute(&app.pool)
        .await
        .unwrap();

    let response = app.post_subscription(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}
