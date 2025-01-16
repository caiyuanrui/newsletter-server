use std::path::Path;

use secrecy::ExposeSecret;

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(Debug, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: secrecy::SecretString,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

/// The configuration file's path is hard coded.
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::from(Path::new(
            "/Users/caiyuanrui/zero2prod/configuration",
        )))
        .build()
        .unwrap()
        .try_deserialize()
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
