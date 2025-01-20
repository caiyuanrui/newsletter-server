use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SubscriberId(String);

impl SubscriberId {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl FromStr for SubscriberId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)?;
        Ok(Self(s.to_owned()))
    }
}
