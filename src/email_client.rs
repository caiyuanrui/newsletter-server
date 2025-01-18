use std::ops::{Deref, DerefMut};

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};

use super::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

pub struct Url(reqwest::Url);

impl Deref for Url {
    type Target = reqwest::Url;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Url {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<&str> for Url {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match reqwest::Url::parse(value) {
            Ok(url) => Ok(Self(url)),
            Err(e) => Err(format!("Failed to parse the url: {}", e)),
        }
    }
}

impl EmailClient {
    pub fn new(base_url: Url, sender: SubscriberEmail, authorization_token: SecretString) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.base_url.join("email").unwrap();
        let request_body = SendEmailRequest {
            from: self.sender.as_ref().to_owned(),
            to: recipient.as_ref().to_owned(),
            subject: subject.to_owned(),
            text_body: text_content.to_owned(),
            html_body: html_content.to_owned(),
            message_stream: None,
        };
        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    text_body: String,
    html_body: String,
    message_stream: Option<String>,
}

#[cfg(test)]
mod tests {

    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use reqwest::Method;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use super::*;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let body: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            match body {
                Ok(body) => {
                    body.get("From").is_some()
                        && body.get("To").is_some()
                        && body.get("Subject").is_some()
                        && body.get("HtmlBody").is_some()
                        && body.get("TextBody").is_some()
                }
                Err(_e) => false,
            }
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let url = mock_server.uri().as_str().try_into().unwrap();
        let email_client = EmailClient::new(
            url,
            sender,
            SecretString::new(Faker.fake::<String>().into_boxed_str()),
        );

        // The MockServer iterates all the mounted mocks to check if the request matched the condition.
        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::path("/email"))
            .and(matchers::method(Method::POST))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert
    }
}
