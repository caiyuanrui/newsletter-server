#[tokio::test]
async fn health_check_works() {
    // Arrage
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn spawn_app() -> String {
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tx.send(format!("http://127.0.0.1:{port}")).unwrap();
        zero2prod::run(listener).await.expect("Server failed.");
    });
    rx.await.unwrap()
}
