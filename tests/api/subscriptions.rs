use crate::Server;

#[tokio::test]
async fn subscribe_returns_200_for_valid_data() {
    let server = Server::spawn().await;

    let url = format!("{}/subscriptions", server.address);
    let body = "name=John%20Doe&email=example%40gmail.com";
    let response = reqwest::Client::new()
        .post(url)
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!(r"select email, name from subscriptions",)
        .fetch_one(&server.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.name, "John Doe");
    assert_eq!(saved.email, "example@gmail.com");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let server = Server::spawn().await;

    let url = format!("{}/subscriptions", server.address);
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=John%20Doe", "form is missing the email"),
        ("email=example%40gmail.com", "form is missing the name"),
        ("", "form is missing both name and email"),
    ];
    for (invalid_body, reason) in test_cases {
        let response = client
            .post(url.clone())
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "Server does not return 400 when form {reason}",
        );
    }
}
