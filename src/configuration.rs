use secrecy::ExposeSecret;

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Debug, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: secrecy::SecretString,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
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
    /// The output goes like "mysql://username:password@host:port/database_name".
    pub fn connection_string(&self) -> secrecy::SecretString {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
        .into()
    }

    /// The output goes like "mysql://username:password@host:port".
    ///
    /// Used for testing.
    pub fn connection_string_without_db(&self) -> secrecy::SecretString {
        format!(
            "mysql://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
        .into()
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
