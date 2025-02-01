use axum::{extract::FromRequestParts, RequestPartsExt};
use tower_sessions::Session;

use crate::domain::SubscriberId;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    /// # Errors
    /// If deleting from the store fails or saving to the store fails, we fail with `Error::Store`.
    pub async fn renew(&self) -> Result<(), tower_sessions::session::Error> {
        self.0.cycle_id().await
    }

    /// # Errors
    /// - This method can fail when `serde_json::to_value` fails.
    /// - If the session has not been hydrated and loading from the store fails, we fail with `Error::Store`.
    pub async fn insert_user_id(
        &self,
        user_id: SubscriberId,
    ) -> Result<(), tower_sessions::session::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    pub async fn get_user_id(
        &self,
    ) -> Result<Option<SubscriberId>, tower_sessions::session::Error> {
        self.0
            .get_value(Self::USER_ID_KEY)
            .await?
            .map(serde_json::from_value::<SubscriberId>)
            .transpose()
            .map_err(tower_sessions::session::Error::SerdeJson)
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = <Session as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts.extract::<Session>().await.map(Self)
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
}
