use crate::helpers::spawn_app;

#[tokio::test]
async fn test_subscribtion_200() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let resp = test_app.post_subscribtion(body.into()).await;

    assert_eq!(200, resp.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&test_app.pool)
        .await
        .expect("Failed to fetch users");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn test_422_on_bad_requestr() {
    let test_app = spawn_app().await;
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, reason) in cases.into_iter() {
        let resp = test_app.post_subscribtion(body.into()).await;
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
        let resp = test_app.post_subscribtion(body.into()).await;

        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap();
        assert_eq!(
            422, status,
            "Expected status 400 because of {reason}. Response {text}",
        )
    }
}
