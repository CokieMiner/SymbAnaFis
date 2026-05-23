//! Serialization / deserialization support for [`Scalar`].
//!
//! Enabled with the `serde` feature.

use crate::Scalar;
use crate::number::logic::float_ops;
use crate::number::logic::int_math;
use crate::number::logic::rational_math;
use crate::number::logic::scalar::ScalarRepr;
use ::serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use alloc::string::String;

impl Serialize for Scalar {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ScalarRepr::Int(ref i) => {
                let val = int_math::to_string(i);
                serializer.serialize_str(&val)
            }
            ScalarRepr::Rational(ref rat) => {
                let val = rational_math::to_string(rat);
                serializer.serialize_str(&val)
            }
            ScalarRepr::Float(ref f) => {
                let val = float_ops::to_string(f);
                serializer.serialize_str(&val)
            }
        }
    }
}

impl<'de> Deserialize<'de> for Scalar {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        parse_scalar(&s).ok_or_else(|| {
            let err = alloc::format!("invalid Scalar: {s}");
            de::Error::custom(err)
        })
    }
}

fn parse_scalar(input: &str) -> Option<Scalar> {
    parse_scalar_inner(input.trim())
}

fn parse_scalar_inner(s: &str) -> Option<Scalar> {
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('-') {
        return parse_positive_scalar(rest).map(|v| -v);
    }
    parse_positive_scalar(s)
}

fn parse_positive_scalar(s: &str) -> Option<Scalar> {
    if s.contains('.') || s.contains('e') || s.contains('E') {
        let f = float_ops::from_str(s)?;
        Some(Scalar::from_float(f))
    } else if let Some((num_str, den_str)) = s.split_once('/') {
        let num = int_math::from_str(num_str)?;
        let den = int_math::from_str(den_str)?;
        if int_math::is_zero(&den) {
            return None;
        }
        Some(Scalar::from_rational(rational_math::new(num, den)))
    } else {
        let i = int_math::from_str(s)?;
        Some(Scalar::from_int(i))
    }
}
