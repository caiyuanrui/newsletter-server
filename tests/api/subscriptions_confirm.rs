use hyper::StatusCode;
use wiremock::{matchers, Mock, ResponseTemplate};

use super::helpers::*;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() -> Result {
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address)).await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() -> Result {
    let test_app = spawn_app().await;

    // Mock a user with pending status in the databse.
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body)?;

    // Extract the link from one of the request fields
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(1, links.len());
        links[0].as_str().to_owned()
    };

    let raw_confirmation_link = get_link(body["HtmlBody"].as_str().unwrap());
    let mut confirmation_link = reqwest::Url::parse(&raw_confirmation_link)?;
    confirmation_link.set_port(Some(test_app.port)).unwrap();

    assert_eq!(confirmation_link.host_str(), Some("127.0.0.1"));

    let response = reqwest::get(confirmation_link).await?;

    assert_eq!(StatusCode::OK, response.status());

    Ok(())
}
