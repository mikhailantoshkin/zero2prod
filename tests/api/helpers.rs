use std::time::Duration;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_config, DatabaseSettings},
    startup::{get_connection_pool, Application},
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

impl TestApp {
    pub async fn post_subscribtion(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subsriptions", self.addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .expect("Failed to send the request")
    }
}

pub async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut conn = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Unable to connect to DB");
    conn.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to DB");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_config().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.app.port = 0;
        c
    };
    // Create and migrate the database
    configure_db(&configuration.database).await;
    // Launch the application as a background task
    let app = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let addr = format!("http://{}:{}", app.addr(), app.port());
    let _ = tokio::spawn(app.run_forever());
    TestApp {
        // How do we get these?
        addr: addr,
        pool: get_connection_pool(&configuration.database).await.unwrap(),
    }
}
