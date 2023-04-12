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

use bytes::BufMut;
use serde::{ser, Serialize};

#[cfg(feature = "decimal")]
use crate::decimal::Decimal;
use crate::error::{Error, Result};

/// A structure for serializing Rust values into a memcomparable bytes.
pub struct Serializer<B: BufMut> {
    output: MaybeFlip<B>,
}

impl<B: BufMut> Serializer<B> {
    /// Create a new `Serializer`.
    pub fn new(buffer: B) -> Self {
        Serializer {
            output: MaybeFlip {
                output: buffer,
                flip: false,
            },
        }
    }

    /// Unwrap the inner buffer from the `Serializer`.
    pub fn into_inner(self) -> B {
        self.output.output
    }

    /// Set whether data is serialized in reverse order.
    pub fn set_reverse(&mut self, reverse: bool) {
        self.output.flip = reverse;
    }
}

/// Serialize the given data structure as a memcomparable byte vector.
pub fn to_vec(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut serializer = Serializer::new(vec![]);
    value.serialize(&mut serializer)?;
    Ok(serializer.into_inner())
}

/// A wrapper around `BufMut` that can flip bits when putting data.
struct MaybeFlip<B: BufMut> {
    output: B,
    flip: bool,
}

macro_rules! def_method {
    ($name:ident, $ty:ty) => {
        fn $name(&mut self, value: $ty) {
            self.output.$name(if self.flip { !value } else { value });
        }
    };
}

impl<B: BufMut> MaybeFlip<B> {
    def_method!(put_u8, u8);

    def_method!(put_u16, u16);

    def_method!(put_u32, u32);

    def_method!(put_u64, u64);

    def_method!(put_u128, u128);

    fn put_slice(&mut self, src: &[u8]) {
        for &val in src {
            let val = if self.flip { !val } else { val };
            self.output.put_u8(val);
        }
    }

    fn put_bytes(&mut self, val: u8, cnt: usize) {
        let val = if self.flip { !val } else { val };
        self.output.put_bytes(val, cnt);
    }
}

// Format Reference:
// https://github.com/facebook/mysql-5.6/wiki/MyRocks-record-format#memcomparable-format
// https://haxisnake.github.io/2020/11/06/TIDB源码学习笔记-基本类型编解码方案/
impl<'a, B: BufMut> ser::Serializer for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = Self;
    type SerializeSeq = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u8(v as u8)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        let u = v as u8 ^ (1 << 7);
        self.serialize_u8(u)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        let u = v as u16 ^ (1 << 15);
        self.serialize_u16(u)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        let u = v as u32 ^ (1 << 31);
        self.serialize_u32(u)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let u = v as u64 ^ (1 << 63);
        self.serialize_u64(u)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        let u = v as u128 ^ (1 << 127);
        self.serialize_u128(u)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.put_u8(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output.put_u16(v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.put_u32(v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.put_u64(v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.output.put_u128(v);
        Ok(())
    }

    fn serialize_f32(self, mut v: f32) -> Result<()> {
        if v.is_nan() {
            v = f32::NAN; // normalize pos/neg NaN
        } else if v == 0.0 {
            v = 0.0; // normalize pos/neg zero
        }
        let u = v.to_bits();
        let u = if v.is_sign_positive() {
            u | (1 << 31)
        } else {
            !u
        };
        self.output.put_u32(u);
        Ok(())
    }

    fn serialize_f64(self, mut v: f64) -> Result<()> {
        if v.is_nan() {
            v = f64::NAN; // normalize pos/neg NaN
        } else if v == 0.0 {
            v = 0.0; // normalize pos/neg zero
        }
        let u = v.to_bits();
        let u = if v.is_sign_positive() {
            u | (1 << 63)
        } else {
            !u
        };
        self.output.put_u64(u);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.output.put_u8(!v.is_empty() as u8);
        let mut len = 0;
        for chunk in v.chunks(8) {
            self.output.put_slice(chunk);
            if chunk.len() != 8 {
                self.output.put_bytes(0, 8 - chunk.len());
            }
            len += chunk.len();
            // append an extra byte that signals the number of significant bytes in this chunk
            // 1-8: many bytes were significant and this group is the last group
            // 9: all 8 bytes were significant and there is more data to come
            let extra = if len == v.len() { chunk.len() as u8 } else { 9 };
            self.output.put_u8(extra);
        }
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_u8(0)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_u8(1)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        assert!(variant_index <= u8::MAX as u32, "too many variants");
        self.serialize_u8(variant_index as u8)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        assert!(variant_index <= u8::MAX as u32, "too many variants");
        self.serialize_u8(variant_index as u8)?;
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        assert!(variant_index <= u8::MAX as u32, "too many variants");
        self.serialize_u8(variant_index as u8)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::NotSupported("map"))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        assert!(variant_index <= u8::MAX as u32, "too many variants");
        self.serialize_u8(variant_index as u8)?;
        Ok(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'a, B: BufMut> ser::SerializeSeq for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        use serde::Serializer;
        self.serialize_u8(1)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        use serde::Serializer;
        self.serialize_u8(0)?;
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeTuple for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeTupleStruct for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeTupleVariant for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeMap for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeStruct for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, B: BufMut> ser::SerializeStructVariant for &'a mut Serializer<B> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<B: BufMut> Serializer<B> {
    /// Serialize a decimal value.
    ///
    /// The encoding format follows `SQLite`: <https://sqlite.org/src4/doc/trunk/www/key_encoding.wiki>
    ///
    /// # Example
    /// ```
    /// use memcomparable::Decimal;
    ///
    /// let d1 = Decimal::Normalized("12.34".parse().unwrap());
    /// let d2 = Decimal::Inf;
    ///
    /// let mut ser = memcomparable::Serializer::new(vec![]);
    /// ser.serialize_decimal(d1.into()).unwrap();
    /// ser.serialize_decimal(d2).unwrap();
    /// ```
    #[cfg(feature = "decimal")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decimal")))]
    pub fn serialize_decimal(&mut self, decimal: Decimal) -> Result<()> {
        let decimal = match decimal {
            Decimal::NaN => {
                self.output.put_u8(0x06);
                return Ok(());
            }
            Decimal::NegInf => {
                self.output.put_u8(0x07);
                return Ok(());
            }
            Decimal::Inf => {
                self.output.put_u8(0x23);
                return Ok(());
            }
            Decimal::Normalized(d) if d.is_zero() => {
                self.output.put_u8(0x15);
                return Ok(());
            }
            Decimal::Normalized(d) => d,
        };
        let (exponent, significand) = Self::decimal_e_m(decimal);
        if decimal.is_sign_positive() {
            match exponent {
                11.. => {
                    self.output.put_u8(0x22);
                    self.output.put_u8(exponent as u8);
                }
                0..=10 => {
                    self.output.put_u8(0x17 + exponent as u8);
                }
                _ => {
                    self.output.put_u8(0x16);
                    self.output.put_u8(!(-exponent) as u8);
                }
            }
            self.output.put_slice(&significand);
        } else {
            match exponent {
                11.. => {
                    self.output.put_u8(0x8);
                    self.output.put_u8(!exponent as u8);
                }
                0..=10 => {
                    self.output.put_u8(0x13 - exponent as u8);
                }
                _ => {
                    self.output.put_u8(0x14);
                    self.output.put_u8(-exponent as u8);
                }
            }
            for b in significand {
                self.output.put_u8(!b);
            }
        }
        Ok(())
    }

    /// Get the exponent and significand mantissa from a decimal.
    #[cfg(feature = "decimal")]
    fn decimal_e_m(decimal: rust_decimal::Decimal) -> (i8, Vec<u8>) {
        if decimal.is_zero() {
            return (0, vec![]);
        }
        const POW10: [u128; 30] = [
            1,
            10,
            100,
            1000,
            10000,
            100000,
            1000000,
            10000000,
            100000000,
            1000000000,
            10000000000,
            100000000000,
            1000000000000,
            10000000000000,
            100000000000000,
            1000000000000000,
            10000000000000000,
            100000000000000000,
            1000000000000000000,
            10000000000000000000,
            100000000000000000000,
            1000000000000000000000,
            10000000000000000000000,
            100000000000000000000000,
            1000000000000000000000000,
            10000000000000000000000000,
            100000000000000000000000000,
            1000000000000000000000000000,
            10000000000000000000000000000,
            100000000000000000000000000000,
        ];
        let mut mantissa = decimal.mantissa().unsigned_abs();
        let prec = POW10.as_slice().partition_point(|&p| p <= mantissa);

        let e10 = prec as i32 - decimal.scale() as i32;
        let e100 = if e10 >= 0 { (e10 + 1) / 2 } else { e10 / 2 };
        // Maybe need to add a zero at the beginning.
        // e.g. 111.11 -> 2(exponent which is 100 based) + 0.011111(mantissa).
        // So, the `digit_num` of 111.11 will be 6.
        let mut digit_num = if e10 == 2 * e100 { prec } else { prec + 1 };

        let mut byte_array = Vec::with_capacity(16);
        // Remove trailing zero.
        while mantissa % 10 == 0 && mantissa != 0 {
            mantissa /= 10;
            digit_num -= 1;
        }

        // Cases like: 0.12345, not 0.01111.
        if digit_num % 2 == 1 {
            mantissa *= 10;
            // digit_num += 1;
        }
        while mantissa >> 64 != 0 {
            let byte = (mantissa % 100) as u8 * 2 + 1;
            byte_array.push(byte);
            mantissa /= 100;
        }
        // optimize for division
        let mut mantissa = mantissa as u64;
        while mantissa != 0 {
            let byte = (mantissa % 100) as u8 * 2 + 1;
            byte_array.push(byte);
            mantissa /= 100;
        }
        byte_array[0] -= 1;
        byte_array.reverse();

        (e100 as i8, byte_array)
    }
}

#[cfg(test)]
mod tests {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use serde::Serialize;

    use super::*;

    #[test]
    fn test_unit() {
        assert_eq!(to_vec(&()).unwrap(), []);

        #[derive(Serialize)]
        struct UnitStruct;
        assert_eq!(to_vec(&UnitStruct).unwrap(), []);
    }

    #[test]
    fn test_option() {
        assert_eq!(to_vec(&(None as Option<u8>)).unwrap(), [0]);
        assert_eq!(to_vec(&Some(0x12u8)).unwrap(), [1, 0x12]);
    }

    #[test]
    fn test_tuple() {
        let tuple: (i8, i16, i32, i64, i128) = (
            0x12,
            0x1234,
            0x12345678,
            0x1234_5678_8765_4321,
            0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        );
        assert_eq!(
            to_vec(&tuple).unwrap(),
            [
                0x92, 0x92, 0x34, 0x92, 0x34, 0x56, 0x78, 0x92, 0x34, 0x56, 0x78, 0x87, 0x65, 0x43,
                0x21, 0x81, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76,
                0x54, 0x32, 0x10,
            ]
        );

        #[derive(Serialize)]
        struct TupleStruct(u8, u16, u32, u64, u128);
        let tuple = TupleStruct(
            0x12,
            0x1234,
            0x12345678,
            0x1234_5678_8765_4321,
            0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        );
        assert_eq!(
            to_vec(&tuple).unwrap(),
            [
                0x12, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78, 0x87, 0x65, 0x43,
                0x21, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76,
                0x54, 0x32, 0x10,
            ]
        );

        #[derive(Serialize)]
        struct NewTypeStruct(char);
        let tuple = NewTypeStruct('G');
        assert_eq!(to_vec(&tuple).unwrap(), [0, 0, 0, b'G']);
    }

    #[test]
    fn test_vec() {
        let s: &[u8] = &[1, 2, 3];
        assert_eq!(to_vec(&s).unwrap(), [1, 0x01, 1, 0x02, 1, 0x03, 0]);
    }

    #[test]
    fn test_enum() {
        #[derive(PartialEq, Eq, PartialOrd, Ord, Serialize)]
        enum TestEnum {
            Unit,
            NewType(u8),
            Tuple(u8, u8),
            Struct { a: u8, b: u8 },
        }

        let test = TestEnum::Unit;
        assert_eq!(to_vec(&test).unwrap(), [0]);

        let test = TestEnum::NewType(0x12);
        assert_eq!(to_vec(&test).unwrap(), [1, 0x12]);

        let test = TestEnum::Tuple(0x12, 0x34);
        assert_eq!(to_vec(&test).unwrap(), [2, 0x12, 0x34]);

        let test = TestEnum::Struct { a: 0x12, b: 0x34 };
        assert_eq!(to_vec(&test).unwrap(), [3, 0x12, 0x34]);
    }

    #[derive(PartialEq, PartialOrd, Serialize)]
    struct Test {
        a: bool,
        b: f32,
        c: f64,
    }

    #[test]
    fn test_struct() {
        let test = Test {
            a: true,
            b: 0.0,
            c: 0.0,
        };
        assert_eq!(
            to_vec(&test).unwrap(),
            [1, 0x80, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_struct_order() {
        for _ in 0..1000 {
            let mut rng = rand::thread_rng();
            let a = Test {
                a: rng.gen(),
                b: rng.gen(),
                c: rng.gen(),
            };
            let b = Test {
                a: if rng.gen_bool(0.5) { a.a } else { rng.gen() },
                b: if rng.gen_bool(0.5) {
                    a.b
                } else {
                    rng.gen_range(-1.0..1.0)
                },
                c: if rng.gen_bool(0.5) {
                    a.c
                } else {
                    rng.gen_range(-1.0..1.0)
                },
            };
            let ea = to_vec(&a).unwrap();
            let eb = to_vec(&b).unwrap();
            assert_eq!(a.partial_cmp(&b), ea.partial_cmp(&eb));
        }
    }

    #[test]
    fn test_string() {
        assert_eq!(to_vec(&"").unwrap(), [0]);
        assert_eq!(
            to_vec(&"123").unwrap(),
            [1, b'1', b'2', b'3', 0, 0, 0, 0, 0, 3]
        );
        assert_eq!(
            to_vec(&"12345678").unwrap(),
            [1, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', 8]
        );
        assert_eq!(
            to_vec(&"1234567890").unwrap(),
            [
                1, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', 9, b'9', b'0', 0, 0, 0, 0, 0, 0,
                2
            ]
        );
    }

    #[test]
    fn test_string_order() {
        fn to_vec_desc(s: &str) -> Vec<u8> {
            let mut ser = Serializer::new(vec![]);
            ser.set_reverse(true);
            s.serialize(&mut ser).unwrap();
            ser.into_inner()
        }

        for _ in 0..1000 {
            let s = rand_string(0..16);
            let a = s.clone() + &rand_string(0..16);
            let b = s + &rand_string(0..16);

            let ea = to_vec(&a).unwrap();
            let eb = to_vec(&b).unwrap();
            assert_eq!(a.cmp(&b), ea.cmp(&eb));

            let ra = to_vec_desc(&a);
            let rb = to_vec_desc(&b);
            assert_eq!(a.cmp(&b), ra.cmp(&rb).reverse());
        }
    }

    fn rand_string(len_range: std::ops::Range<usize>) -> String {
        let mut rng = rand::thread_rng();
        let len = rng.gen_range(len_range);
        rng.sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }

    #[test]
    #[cfg(feature = "decimal")]
    fn test_decimal_e_m() {
        // from: https://sqlite.org/src4/doc/trunk/www/key_encoding.wiki
        let cases = vec![
            // (decimal, exponents, significand)
            ("1.0", 1, "02"),
            ("10.0", 1, "14"),
            ("10", 1, "14"),
            ("99.0", 1, "c6"),
            ("99.01", 1, "c7 02"),
            ("99.0001", 1, "c7 01 02"),
            ("100.0", 2, "02"),
            ("100.01", 2, "03 01 02"),
            ("100.1", 2, "03 01 14"),
            ("111.11", 2, "03 17 16"),
            ("1234", 2, "19 44"),
            ("9999", 2, "c7 c6"),
            ("9999.000001", 2, "c7 c7 01 01 02"),
            ("9999.000009", 2, "c7 c7 01 01 12"),
            ("9999.00001", 2, "c7 c7 01 01 14"),
            ("9999.00009", 2, "c7 c7 01 01 b4"),
            ("9999.000099", 2, "c7 c7 01 01 c6"),
            ("9999.0001", 2, "c7 c7 01 02"),
            ("9999.001", 2, "c7 c7 01 14"),
            ("9999.01", 2, "c7 c7 02"),
            ("9999.1", 2, "c7 c7 14"),
            ("10000", 3, "02"),
            ("10001", 3, "03 01 02"),
            ("12345", 3, "03 2f 5a"),
            ("123450", 3, "19 45 64"),
            ("1234.5", 2, "19 45 64"),
            ("12.345", 1, "19 45 64"),
            ("0.123", 0, "19 3c"),
            ("0.0123", 0, "03 2e"),
            ("0.00123", -1, "19 3c"),
            ("9223372036854775807", 10, "13 2d 43 91 07 89 6d 9b 75 0e"),
        ];

        for (decimal, exponents, significand) in cases {
            let d = decimal.parse::<rust_decimal::Decimal>().unwrap();
            let (exp, sig) = Serializer::<Vec<u8>>::decimal_e_m(d);
            assert_eq!(exp, exponents, "wrong exponents for decimal: {decimal}");
            assert_eq!(
                sig.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                significand,
                "wrong significand for decimal: {decimal}"
            );
        }
    }

    #[test]
    fn test_reverse_order() {
        // Order: (ASC, DESC)
        let v1 = (0u8, 1i32);
        let v2 = (0u8, -1i32);
        let v3 = (1u8, -1i32);

        fn serialize(v: (u8, i32)) -> Vec<u8> {
            let mut ser = Serializer::new(vec![]);
            v.0.serialize(&mut ser).unwrap();
            ser.set_reverse(true);
            v.1.serialize(&mut ser).unwrap();
            ser.into_inner()
        }
        assert!(serialize(v1) < serialize(v2));
        assert!(serialize(v2) < serialize(v3));
    }
}
