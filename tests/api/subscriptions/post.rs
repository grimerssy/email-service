use crate::TestServer;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[proc::test]
async fn sends_an_email_for_valid_data(server: TestServer) {
    let body = "name=John%20Doe&email=example%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server.email_server)
        .await;

    server.post_supscriptions(body.into()).await;
}

#[proc::test]
async fn returns_200_for_valid_data(server: TestServer) {
    let body = "name=John%20Doe&email=example%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server.email_server)
        .await;

    let response = server.post_supscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!(r"select email, name from subscriptions",)
        .fetch_all(&server.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.len(), 1);
    let saved = saved.first().unwrap();
    assert_eq!(saved.name, "John Doe");
    assert_eq!(saved.email, "example@gmail.com");
}

#[proc::test]
async fn returns_400_when_data_is_missing(server: TestServer) {
    let test_cases = vec![
        ("name=John%20Doe", "form is missing the email"),
        ("email=example%40gmail.com", "form is missing the name"),
        ("", "form is missing both name and email"),
    ];
    for (invalid_body, reason) in test_cases {
        let response = server.post_supscriptions(invalid_body.into()).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}

#[proc::test]
async fn returns_400_when_data_is_invalid(server: TestServer) {
    let test_cases = vec![
        ("name=&email=example%40gmail.com", "has empty name"),
        ("name=John%20Doe&email=", "has empty email"),
        (
            "name=John%20Doe&email=definitely-not-an-email",
            "has invalid email",
        ),
    ];
    for (invalid_body, reason) in test_cases {
        let response = server.post_supscriptions(invalid_body.into()).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "Server does not return 400 when form {reason}",
        );
    }
}
