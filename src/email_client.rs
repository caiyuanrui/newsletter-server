use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

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
    pub fn new(
        base_url: Url,
        sender: SubscriberEmail,
        authorization_token: SecretString,
        time_out: Duration,
    ) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(time_out)
                .build()
                .expect("Failed to build the email client"),
            base_url,
            sender,
            authorization_token,
        }
    }

    /// # Errors
    ///
    /// This function will timeout if 10 seconds has elasped.
    #[instrument(name = "Send email with Postmark ", skip(self), fields(subscriber_email = %recipient))]
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        use hyper::header::*;

        let url = self.base_url.join("email").unwrap();
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            text_body: text_content,
            html_body: html_content,
        };

        let headers: HeaderMap = HeaderMap::from_iter(
            [
                (CONTENT_TYPE, "application/json".parse().unwrap()),
                (ACCEPT, "application/json".parse().unwrap()),
                (
                    "X-Postmark-Server-Token".parse().unwrap(),
                    self.authorization_token.expose_secret().parse().unwrap(),
                ),
            ]
            .into_iter(),
        );

        self.http_client
            .post(url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?
            .error_for_status()
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[cfg(test)]
mod tests {

    use claim::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use reqwest::Method;
    use std::time::Duration;
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

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: Url) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            SecretString::new(Faker.fake::<String>().into_boxed_str()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().as_str().try_into().unwrap());

        // The MockServer iterates all the mounted mocks to check if the request matched the condition.
        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method(Method::POST))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content = content();
        let _ = email_client
            .send_email(&email(), &subject(), &content, &content)
            .await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().as_str().try_into().unwrap());

        // The MockServer iterates all the mounted mocks to check if the request matched the condition.
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content = content();
        let outcome = email_client
            .send_email(&email(), &subject(), &content, &content)
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_failes_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().as_str().try_into().unwrap());

        // The MockServer iterates all the mounted mocks to check if the request matched the condition.
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content = content();
        let outcome = email_client
            .send_email(&email(), &subject(), &content, &content)
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().as_str().try_into().unwrap());

        // The MockServer iterates all the mounted mocks to check if the request matched the condition.
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let content = content();

        let outcome = email_client
            .send_email(&email(), &subject(), &content, &content)
            .await;

        // Assert
        assert_err!(outcome);
    }
}
