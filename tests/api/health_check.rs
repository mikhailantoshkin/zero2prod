use std::time::Duration;

use crate::helpers::spawn_app;

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
