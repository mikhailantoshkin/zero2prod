use std::time::Duration;

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
    let port = listener
        .local_addr()
        .expect("Unable to get local addr")
        .port();
    let server = zero2prod::run(listener).expect("Unable to start server");
    tokio::spawn(server);
    format!("http://localhost:{}", port)
}
