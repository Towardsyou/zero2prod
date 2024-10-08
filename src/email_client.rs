use validator::ValidateUrl;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: reqwest::Client,
    sender: SubscriberEmail,
    api_url: String,
}

impl EmailClient {
    pub fn new(
        sender: SubscriberEmail,
        api_url: String,
    ) -> Result<Self, String> {
        if api_url.trim().validate_url() {
            Ok(Self {
                http_client: reqwest::Client::new(),
                sender,
                api_url,
            })
        } else {
            Err("Invalid API URL {api_url}".to_string())
        }
    }

    pub async fn SendEmail(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_context: &str,
    ) -> Result<(), String> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_valid_api_url_pass() {
        let sender = SubscriberEmail::from_str("test@example.com").unwrap();
        assert!(EmailClient::new(sender, "https://example.com".to_string()).is_ok());
    }

    #[test]
    fn test_invalid_api_url_failed() {
        let sender = SubscriberEmail::from_str("test@example.com").unwrap();
        assert!(EmailClient::new(sender, ":/http;example.com".to_string()).is_err());
    }
}
