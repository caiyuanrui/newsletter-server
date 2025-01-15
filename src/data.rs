use std::{ops::Deref, sync::Arc};

use serde::{Deserialize, Serialize};

#[doc(alias = "state")]
#[derive(Debug)]
pub struct Data<T: ?Sized>(Arc<T>);

impl<T> Data<T> {
    pub fn new(state: T) -> Data<T> {
        Data(Arc::new(state))
    }
}

impl<T: ?Sized> Data<T> {
    pub fn get_ref(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T: ?Sized> Deref for Data<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Self {
        Data(self.0.clone())
    }
}

impl<T: ?Sized> From<Arc<T>> for Data<T> {
    fn from(value: Arc<T>) -> Self {
        Data(value)
    }
}

impl<T: Default> Default for Data<T> {
    fn default() -> Self {
        Data::new(T::default())
    }
}

impl<T: Serialize> Serialize for Data<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Data<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Data::new(T::deserialize(deserializer)?))
    }
}
