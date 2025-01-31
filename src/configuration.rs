use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde_aux::field_attributes::{deserialize_bool_from_anything, deserialize_number_from_string};
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};

use crate::{appstate::HmacSecret, domain::SubscriberEmail};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: SecretString,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: secrecy::SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    #[serde(deserialize_with = "deserialize_bool_from_anything")]
    pub require_ssl: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: HmacSecret,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    sender_email: String,
    pub authorization_token: SecretString,
    timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(true))
        .add_source(
            config::File::from(configuration_directory.join(environment.as_str())).required(true),
        )
        .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()
        .unwrap()
        .try_deserialize()
}

pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a valid environment. Use either `development` or `production`.",
                other
            )),
        }
    }
}

impl DatabaseSettings {
    /// Used for testing
    pub fn without_db(&self) -> MySqlConnectOptions {
        let ssl_mod = if self.require_ssl {
            MySqlSslMode::Required
        } else {
            MySqlSslMode::Preferred
        };
        MySqlConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mod)
    }

    pub fn with_db(&self) -> MySqlConnectOptions {
        use sqlx::ConnectOptions;
        use tracing::log::LevelFilter;

        self.without_db()
            .database(&self.database_name)
            .log_statements(LevelFilter::Trace)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static MU: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_configuration_with_production_env() {
        let _guard = MU.lock().unwrap();
        std::env::set_var("APP_ENVIRONMENT", "production");

        assert_eq!(
            "production",
            std::env::var("APP_ENVIRONMENT").expect("Failed to set env in test")
        );

        let configuration = get_configuration().expect("Failed to get configuration");
        assert_eq!("0.0.0.0", configuration.application.host);
        assert_eq!(8000, configuration.application.port);
    }

    #[test]
    fn test_get_configuration_with_development_env() {
        let _guard = MU.lock().unwrap();

        std::env::set_var("APP_ENVIRONMENT", "development");

        assert_eq!(
            "development",
            std::env::var("APP_ENVIRONMENT").expect("Failed to set env in test")
        );

        let configuration = get_configuration().expect("Failed to get configuration");
        assert_eq!("127.0.0.1", configuration.application.host);
        assert_eq!(8000, configuration.application.port);
    }
}
