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

//! A memcomparable serialization format.
//!
//! The memcomparable format allows comparison of two values by using the simple memcmp function.
//!
//! # Usage
//!
//! ```
//! // serialize
//! let key1 = memcomparable::to_vec(&"hello").unwrap();
//! let key2 = memcomparable::to_vec(&"world").unwrap();
//! assert!(key1 < key2);
//!
//! // deserialize
//! let v1: String = memcomparable::from_slice(&key1).unwrap();
//! let v2: String = memcomparable::from_slice(&key2).unwrap();
//! assert_eq!(v1, "hello");
//! assert_eq!(v2, "world");
//! ```
//!
//! # Features
//!
//! - `decimal`: Enable (de)serialization for [`Decimal`] type.
//!     - [`Serializer::serialize_decimal`]
//!     - [`Deserializer::deserialize_decimal`]
//!
//! # Format
//!
//! The serialization format follows [MySQL's memcomparable format](https://github.com/facebook/mysql-5.6/wiki/MyRocks-record-format#memcomparable-format).
//!
//! | Type                                          | Length (bytes)       |
//! | --------------------------------------------- | -------------------- |
//! | `bool`                                        | 1                    |
//! | `char`                                        | 4                    |
//! | `i8`/`i16`/`i32`/`i64`/`u8`/`u16`/`u32`/`u64` | 1/2/4/8              |
//! | `f32`/`f64`                                   | 4/8                  |
//! | `Decimal`                                     | Variable             |
//! | `str`/`bytes`                                 | (L + 7) / 8 x 9      |
//! | `Option<T>`                                   | 1 + len(T)           |
//! | `&[T]`                                        | (1 + len(T)) x L + 1 |
//! | `(T1, T2, ..)`                                | sum(len(Ti))         |
//! | `struct { a: T1, b: T2, .. }`                 | sum(len(Ti))         |
//! | `enum { V1, V2, .. }`                         | 1 + len(Vi)          |
//!
//! **WARN: The format is not guaranteed to be stable in minor version change, e.g. 0.1 -> 0.2.**

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod de;
#[cfg(feature = "decimal")]
mod decimal;
mod error;
mod ser;

pub use de::{from_slice, Deserializer};
#[cfg(feature = "decimal")]
pub use decimal::Decimal;
pub use error::{Error, Result};
pub use ser::{to_vec, Serializer};
