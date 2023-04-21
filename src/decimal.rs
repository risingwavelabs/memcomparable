// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{Deserializer, Error, Serializer};
use std::fmt::Display;
use std::str::FromStr;

/// An extended decimal number with `NaN`, `-Inf` and `Inf`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(docsrs, doc(cfg(feature = "decimal")))]
pub enum Decimal {
    /// Negative infinity.
    NegInf,
    /// Normalized value.
    Normalized(rust_decimal::Decimal),
    /// Infinity.
    Inf,
    /// Not a Number.
    NaN,
}

impl Decimal {
    /// A constant representing 0.
    pub const ZERO: Self = Decimal::Normalized(rust_decimal::Decimal::ZERO);

    /// Serialize the decimal into a vector.
    pub fn to_vec(&self) -> crate::Result<Vec<u8>> {
        let mut serializer = Serializer::new(vec![]);
        serializer.serialize_decimal(*self)?;
        Ok(serializer.into_inner())
    }

    /// Deserialize a decimal value from a memcomparable bytes.
    pub fn from_slice(bytes: &[u8]) -> crate::Result<Self> {
        let mut deserializer = Deserializer::new(bytes);
        let t = deserializer.deserialize_decimal()?;
        if !deserializer.has_remaining() {
            Ok(t)
        } else {
            Err(Error::TrailingCharacters)
        }
    }
}

impl From<rust_decimal::Decimal> for Decimal {
    fn from(decimal: rust_decimal::Decimal) -> Self {
        Decimal::Normalized(decimal)
    }
}

impl FromStr for Decimal {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nan" | "NaN" => Ok(Decimal::NaN),
            "-inf" | "-Inf" => Ok(Decimal::NegInf),
            "inf" | "Inf" => Ok(Decimal::Inf),
            _ => Ok(Decimal::Normalized(s.parse()?)),
        }
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decimal::NaN => write!(f, "NaN"),
            Decimal::NegInf => write!(f, "-Inf"),
            Decimal::Inf => write!(f, "Inf"),
            Decimal::Normalized(n) => write!(f, "{}", n),
        }
    }
}
