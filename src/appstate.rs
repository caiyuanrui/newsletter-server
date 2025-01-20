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
