use secrecy::ExposeSecret;
use tokio::net::TcpListener;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
    telementry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_pool =
        sqlx::MySqlPool::connect(configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to connect to MySQL.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;
    run(listener, db_pool).await
}
