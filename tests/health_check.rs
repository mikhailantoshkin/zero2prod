use std::time::Duration;

use sqlx::PgPool;
use zero2prod::configuration::get_config;

#[tokio::test]
async fn health_check_test() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/health_check", addr))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("Failed to send a request");
    assert!(resp.status().is_success());
    assert_eq!(resp.content_length(), Some(0));
}

async fn spawn_app() -> String {
    let listener = std::net::TcpListener::bind("0.0.0.0:0").expect("Unable to bind to a socket");
    let config = get_config().expect("Failed to read configuration");
    let conn_str = config.database.connfection_string();
    let port = listener
        .local_addr()
        .expect("Unable to get local addr")
        .port();
    let conn = PgPool::connect(&conn_str)
        .await
        .expect("Unable to connecto to database");
    let server = zero2prod::startup::run(listener, conn).expect("Unable to start server");
    tokio::spawn(server);
    format!("http://localhost:{}", port)
}

#[tokio::test]
async fn test_subscribtion_200() {
    let addr = spawn_app().await;
    let config = get_config().expect("Failed to read configuration");
    let conn_str = config.database.connfection_string();
    let conn = PgPool::connect(&conn_str)
        .await
        .expect("Unable to connecto to database");
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let resp = client
        .post(format!("{}/subsriptions", addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("Failed to send the request");
    assert_eq!(200, resp.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&conn)
        .await
        .expect("Failed to fetch users");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn test_400_on_bad_requestr() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, reason) in cases.into_iter() {
        let resp = client
            .post(format!("{}/subsriptions", addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .expect("Failed to send the request");
        assert_eq!(
            422,
            resp.status().as_u16(),
            "Expected status 400 because of {reason}",
        )
    }
}
