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
    let server = zero2prod::startup::run(listener).expect("Unable to start server");
    tokio::spawn(server);
    format!("http://localhost:{}", port)
}

#[tokio::test]
async fn test_subscribtion_200() {
    let addr = spawn_app().await;
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
