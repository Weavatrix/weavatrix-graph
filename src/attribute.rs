use crate::{String, Vec};
use alloc::collections::BTreeMap;
use core::hash::{Hash, Hasher};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Deterministic JSON-like attribute value for graph extensions.
///
/// The graph core keeps attributes typed enough to preserve booleans, numeric
/// counters, nested ranges, and arrays without taking a JSON runtime dependency.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// Creates a finite floating-point attribute value.
    ///
    /// # Errors
    ///
    /// Returns an error for NaN and infinity, which are not valid JSON numbers.
    pub fn new(value: f64) -> Result<Self, String> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(format!("float attribute must be finite: {value}"))
        }
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for FiniteF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FiniteF64 {}

impl PartialOrd for FiniteF64 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FiniteF64 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for FiniteF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Deterministic JSON-like attribute value for graph extensions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Float(FiniteF64),
    String(String),
    List(Vec<AttributeValue>),
    Object(BTreeMap<String, AttributeValue>),
}

impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for AttributeValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u64> for AttributeValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<i32> for AttributeValue {
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<u32> for AttributeValue {
    fn from(value: u32) -> Self {
        Self::Unsigned(u64::from(value))
    }
}

impl TryFrom<f64> for AttributeValue {
    type Error = String;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Ok(Self::Float(FiniteF64::new(value)?))
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        Self::String(String::from(value))
    }
}

impl From<Vec<AttributeValue>> for AttributeValue {
    fn from(value: Vec<AttributeValue>) -> Self {
        Self::List(value)
    }
}

impl From<BTreeMap<String, AttributeValue>> for AttributeValue {
    fn from(value: BTreeMap<String, AttributeValue>) -> Self {
        Self::Object(value)
    }
}
