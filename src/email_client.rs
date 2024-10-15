use secrecy::{ExposeSecret, Secret};
use validator::ValidateUrl;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: reqwest::Client,
    sender: SubscriberEmail,
    api_url: String,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        sender: SubscriberEmail,
        api_url: String,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Result<Self, String> {
        if api_url.trim().validate_url() {
            let http_client: reqwest::Client = reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("fail to build email client");
            Ok(Self {
                http_client,
                sender,
                api_url,
                authorization_token,
            })
        } else {
            Err("Invalid API URL {api_url}".to_string())
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_context: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.api_url);
        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: subject,
            html_body: html_content,
            text_body: text_context,
        };
        self.http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
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
    use claim::{assert_err, assert_ok};
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::{internet::en::SafeEmail, lorem::en::Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::*;
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    fn test_valid_api_url_pass() {
        let sender = SubscriberEmail::from_str("test@example.com").unwrap();
        assert!(EmailClient::new(
            sender,
            "https://example.com".to_string(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
        .is_ok());
    }

    #[test]
    fn test_invalid_api_url_failed() {
        let sender = SubscriberEmail::from_str("test@example.com").unwrap();
        assert!(EmailClient::new(
            sender,
            ":/http;example.com".to_string(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
        .is_err());
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::from_str(&SafeEmail().fake::<String>()).unwrap()
    }

    fn email_client(api_url: String) -> EmailClient {
        EmailClient::new(email(), api_url, Secret::new(Faker.fake()), std::time::Duration::from_millis(200)).unwrap()
    }

    struct SendEmailBodyMathcer;

    impl wiremock::Match for SendEmailBodyMathcer {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_the_api() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMathcer)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Mock expectations are checked on drop
    }

    #[tokio::test]
    async fn send_email_return_ok_when_response_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(resp);
    }

    #[tokio::test]
    async fn send_email_return_error_when_response_5xx() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(resp);
    }

    #[tokio::test]
    async fn send_email_return_error_when_respond_in_180s() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(resp);
    }
}
