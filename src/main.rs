use std::time::Duration;

use secrecy::ExposeSecret;
use sqlx::mysql::MySqlPoolOptions;
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

    let db_pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy(configuration.database.connection_string().expose_secret())
        .expect("Failed to connect to MySQL.");

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).await?;

    println!("server is running on {}", listener.local_addr().unwrap());

    run(listener, db_pool).await
}
