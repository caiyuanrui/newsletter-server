use std::str::FromStr;

use uuid::Uuid;

#[derive(Debug)]
pub struct SubscriberId(Uuid);

impl SubscriberId {
    pub fn into_string(self) -> String {
        self.0.to_string()
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl TryFrom<&str> for SubscriberId {
    type Error = uuid::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Uuid::from_str(value).map(Self)
    }
}

impl From<Uuid> for SubscriberId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<SubscriberId> for String {
    fn from(value: SubscriberId) -> Self {
        value.0.to_string()
    }
}
