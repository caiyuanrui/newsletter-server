use serde::{Deserialize, Serialize};

use core::ops::{Deref, DerefMut};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderPairRecord {
    pub key: String,
    pub value: Vec<u8>,
}

impl<T, U> From<(T, U)> for HeaderPairRecord
where
    T: Into<String>,
    U: Into<Vec<u8>>,
{
    fn from(value: (T, U)) -> Self {
        Self {
            key: value.0.into(),
            value: value.1.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub Vec<HeaderPairRecord>);

impl Headers {
    pub fn new<T, U>(inner: T) -> Self
    where
        T: Into<Vec<U>>,
        U: Into<HeaderPairRecord>,
    {
        Self(inner.into().into_iter().map(Into::into).collect())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.try_into().expect("Failed to serialize Headers")
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bytes.try_into().expect("Failed to deserialize Headers")
    }
}

impl TryFrom<&[u8]> for Headers {
    type Error = bincode::Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes)
    }
}

impl TryFrom<&Headers> for Vec<u8> {
    type Error = bincode::Error;
    fn try_from(value: &Headers) -> Result<Self, Self::Error> {
        bincode::serialize(value)
    }
}

impl TryFrom<Headers> for Vec<u8> {
    type Error = bincode::Error;
    fn try_from(value: Headers) -> Result<Self, Self::Error> {
        bincode::serialize(&value)
    }
}

impl From<&axum::http::HeaderMap> for Headers {
    fn from(value: &axum::http::HeaderMap) -> Self {
        value
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()).into())
            .collect()
    }
}

impl From<axum::http::HeaderMap> for Headers {
    fn from(value: axum::http::HeaderMap) -> Self {
        value
            .into_iter()
            .filter_map(|(name, value)| {
                name.map(|name| HeaderPairRecord {
                    key: name.to_string(),
                    value: value.as_bytes().to_vec(),
                })
            })
            .collect()
    }
}

impl Deref for Headers {
    type Target = Vec<HeaderPairRecord>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Headers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<HeaderPairRecord> for Headers {
    fn from_iter<T: IntoIterator<Item = HeaderPairRecord>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for Headers {
    type Item = HeaderPairRecord;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_headers_as_bytes() {
        let raw_headers = Headers::new([
            ("Content-Type", "application/json"),
            ("Content_length", "0"),
        ]);
        let bytes = raw_headers.to_bytes();
        let new_headers = Headers::from_bytes(&bytes);
        assert_eq!(raw_headers, new_headers);
    }
}
