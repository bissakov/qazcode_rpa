use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

pub const ALPHABET: [char; 62] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct NanoId(Arc<str>);

impl Deref for NanoId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for NanoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for NanoId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NanoId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(NanoId(Arc::from(s)))
    }
}

impl Default for NanoId {
    fn default() -> Self {
        NanoId::new_with_nanoid()
    }
}

impl NanoId {
    pub fn new<S>(s: S) -> Self
    where
        S: AsRef<str>,
    {
        NanoId(Arc::from(s.as_ref()))
    }

    pub fn new_with_nanoid() -> Self {
        NanoId(Arc::from(nanoid!(8, &ALPHABET)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
