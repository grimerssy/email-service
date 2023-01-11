use std::net::TcpListener;

#[tokio::test]
async fn it_works() {
    let addr = spawn_app();
    let url = format!("{addr}/health_check");
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("Failed to execute request.");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_data() {
    let addr = spawn_app();
    let url = format!("{addr}/subscribe");
    let body = "name=John%20Doe&email=example%40gmail.com";
    let response = reqwest::Client::new()
        .post(url)
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let addr = spawn_app();
    let url = format!("{addr}/subscribe");
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
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "Server does not return 400 when form {reason}.",
        );
    }
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
