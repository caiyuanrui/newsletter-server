use sqlx::MySqlPool;

use crate::{email_client::EmailClient, utils::Data};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    pub email_client: Data<EmailClient>,
}
