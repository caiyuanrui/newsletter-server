use std::ops::{Deref, DerefMut};

use sqlx::MySqlPool;

use crate::{email_client::EmailClient, utils::Data};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    pub email_client: Data<EmailClient>,
    pub base_url: ApplicationBaseUrl,
}

#[derive(Debug, Clone)]
pub struct ApplicationBaseUrl(pub String);

impl ApplicationBaseUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for ApplicationBaseUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ApplicationBaseUrl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
