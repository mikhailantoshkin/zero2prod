use std::{str::FromStr, time::Duration};

use crate::domain::SubscriberEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: SubscriberEmail,
    auth_token: Secret<String>,
}

//TODO: thiserror?
impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        auth_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            base_url: Url::from_str(&base_url).unwrap(),
            sender,
            auth_token,
        }
    }
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), anyhow::Error> {
        let url = self.base_url.join("email")?;
        let request_body = SendEmailRequest {
            from: &self.sender,
            to: &recipient,
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        self.http_client
            .post(url)
            .json(&request_body)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a SubscriberEmail,
    to: &'a SubscriberEmail,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct EmailBodyMatcher;
    impl wiremock::Match for EmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From")
                    .and(body.get("To"))
                    .and(body.get("Subject"))
                    .and(body.get("HtmlBody"))
                    .and(body.get("TextBody"))
                    .is_some()
            } else {
                false
            }
        }
    }
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn random_email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_uri: String) -> EmailClient {
        EmailClient::new(
            base_uri,
            random_email(),
            Secret::new(Faker.fake()),
            Duration::from_millis(10),
        )
    }
    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(EmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        let result = email_client(mock_server.uri())
            .send_email(random_email(), &subject(), &content(), &content())
            .await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn send_fails_on_500() {
        let mock_server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        let result = email_client(mock_server.uri())
            .send_email(random_email(), &subject(), &content(), &content())
            .await;
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn send_times_out() {
        let mock_server = MockServer::start().await;
        let response = ResponseTemplate::new(500).set_delay(Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        let result = email_client(mock_server.uri())
            .send_email(random_email(), &subject(), &content(), &content())
            .await;
        assert!(result.is_err());
    }
}
