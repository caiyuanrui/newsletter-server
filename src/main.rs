use std::time::Duration;

use sqlx::mysql::MySqlPoolOptions;
use std::net::TcpListener;

use zero2prod::{
    configuration::get_configuration,
    email_client::EmailClient,
    startup::run,
    telementry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    tracing::info!("{:?}", configuration);

    let db_pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let url = configuration
        .email_client
        .base_url
        .as_str()
        .try_into()
        .expect("Failed to parse the url");
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Failed to parse the email sender's name");
    let timeout = configuration.email_client.timeout();
    let authorization_token = configuration.email_client.authorization_token;
    let email_client = EmailClient::new(url, sender_email, authorization_token, timeout);

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;

    tracing::info!("server is running on {}", listener.local_addr().unwrap());

    run(listener, db_pool, email_client).await
}
