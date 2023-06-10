use std::time::Duration;

use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_config, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".into();
    let subscriber_name = "test".into();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber).expect("Failed to initialize tracing");
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber).expect("Failed to initialize tracing");
    }
});

pub struct TestApp {
    pub pool: PgPool,
    pub addr: String,
}

#[tokio::test]
async fn health_check_test() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/health_check", test_app.addr))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("Failed to send a request");
    assert!(resp.status().is_success());
    assert_eq!(resp.content_length(), Some(0));
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut conn =
        PgConnection::connect(&config.connfection_string_withot_db_name().expose_secret())
            .await
            .expect("Unable to connect to DB");
    conn.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let pool = PgPool::connect(&config.connfection_string().expose_secret())
        .await
        .expect("Failed to connect to DB");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let listener = std::net::TcpListener::bind("0.0.0.0:0").expect("Unable to bind to a socket");
    let port = listener
        .local_addr()
        .expect("Unable to get local addr")
        .port();
    let addr = format!("http://localhost:{}", port);

    let mut config = get_config().expect("Failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_db(&config.database).await;

    let server = zero2prod::startup::run(listener, pool.clone()).expect("Unable to start server");
    tokio::spawn(server);

    TestApp { addr, pool }
}

#[tokio::test]
async fn test_subscribtion_200() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let resp = client
        .post(format!("{}/subsriptions", test_app.addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("Failed to send the request");

    assert_eq!(200, resp.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&test_app.pool)
        .await
        .expect("Failed to fetch users");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn test_400_on_bad_requestr() {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, reason) in cases.into_iter() {
        let resp = client
            .post(format!("{}/subsriptions", test_app.addr))
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
