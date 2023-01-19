use crate::{configuration::EmailClientConfig as Config, domain::SubscriberEmail};
use reqwest::Url;
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct EmailClient {
    http_client: reqwest::Client,
    base_url: Url,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(config: Config) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .unwrap(),
            base_url: config.base_url,
            sender: config.sender,
            authorization_token: config.authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> reqwest::Result<()> {
        let url = self.base_url.join("/email").unwrap();
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body,
            text_body,
        };
        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()
            .map(|_| ())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use std::time::Duration;
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Match, Mock, MockServer, ResponseTemplate,
    };

    fn expected_body() -> impl Match + 'static {
        struct BodyMatcher;
        impl Match for BodyMatcher {
            fn matches(&self, request: &wiremock::Request) -> bool {
                let json = serde_json::from_slice(&request.body);
                if json.is_err() {
                    return false;
                }
                let json: serde_json::Value = json.unwrap();
                json.get("From").is_some()
                    && json.get("To").is_some()
                    && json.get("Subject").is_some()
                    && json.get("HtmlBody").is_some()
                    && json.get("TextBody").is_some()
            }
        }
        BodyMatcher
    }

    async fn configure_server(server: &MockServer, response: ResponseTemplate) {
        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(expected_body())
            .respond_with(response)
            .expect(1)
            .mount(server)
            .await
    }

    async fn send_email(server: &MockServer) -> reqwest::Result<()> {
        let email = || SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject = Sentence(1..2).fake::<String>();
        let content = Paragraph(1..10).fake::<String>();
        let config = Config {
            timeout: Duration::from_millis(200),
            base_url: Url::parse(&server.uri()).unwrap(),
            sender: email(),
            authorization_token: Secret::new(Faker.fake()),
        };
        EmailClient::new(config)
            .send_email(&email(), &subject, &content, &content)
            .await
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let server = MockServer::start().await;
        configure_server(&server, ResponseTemplate::new(200)).await;
        let result = send_email(&server).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_500() {
        let server = MockServer::start().await;
        configure_server(&server, ResponseTemplate::new(500)).await;
        let result = send_email(&server).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_email_fails_if_server_takes_too_long() {
        let server = MockServer::start().await;
        configure_server(
            &server,
            ResponseTemplate::new(200).set_delay(Duration::from_secs(60)),
        )
        .await;
        let result = send_email(&server).await;
        assert!(result.is_err());
    }
}
