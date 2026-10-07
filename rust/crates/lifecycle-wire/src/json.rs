use std::collections::BTreeSet;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::WireError;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate names or unsafe numeric values")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        if value.unsigned_abs() > MAX_SAFE_INTEGER {
            return Err(E::custom("integer exceeds interoperable JSON range"));
        }
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        if value > MAX_SAFE_INTEGER {
            return Err(E::custom("integer exceeds interoperable JSON range"));
        }
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        if !value.is_finite() || (value.fract() == 0.0 && value.abs() > MAX_SAFE_INTEGER as f64) {
            return Err(E::custom("number exceeds interoperable JSON range"));
        }
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictValue)
            .ok_or_else(|| E::custom("non-finite number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut items = Vec::new();
        while let Some(StrictValue(value)) = access.next_element::<StrictValue>()? {
            items.push(value);
        }
        Ok(StrictValue(Value::Array(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut seen = BTreeSet::new();
        let mut items = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            let StrictValue(value) = access.next_value::<StrictValue>()?;
            items.insert(key, value);
        }
        Ok(StrictValue(Value::Object(items)))
    }
}

pub fn parse_strict_json(input: &[u8]) -> Result<Value, WireError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let StrictValue(value) = StrictValue::deserialize(&mut deserializer)
        .map_err(|error| WireError::MalformedJson(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| WireError::MalformedJson(error.to_string()))?;
    Ok(value)
}
