use crate::errors::{Error, ErrorKind};
use crate::parser::{PhpParser, PhpToken, PhpTokenKind};
use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess};

/// A deserializer for PHP serialized data.
#[derive(Debug)]
pub struct PhpDeserializer<'de> {
    parser: PhpParser<'de>,
}

impl<'de> PhpDeserializer<'de> {
    /// Create a new deserializer from a slice of bytes.
    #[must_use]
    pub const fn new(data: &'de [u8]) -> Self {
        PhpDeserializer {
            parser: PhpParser::new(data),
        }
    }

    /// Create a new deserializer from an existing parser.
    ///
    /// This is useful when you have already parsed part of a serialized structure
    /// and want to deserialize the remaining part.
    #[must_use]
    pub const fn from_parser(parser: PhpParser<'de>) -> Self {
        PhpDeserializer { parser }
    }

    /// Consume this deserializer and return the underlying parser.
    ///
    /// This allows you to continue parsing after deserialization is complete.
    #[must_use]
    pub fn into_parser(self) -> PhpParser<'de> {
        self.parser
    }

    fn deserialize_token<V>(&mut self, visitor: V, token: PhpToken<'de>) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match token {
            PhpToken::Null => visitor.visit_unit(),
            PhpToken::Boolean(b) => visitor.visit_bool(b),
            PhpToken::Integer(i) => visitor.visit_i64(i),
            PhpToken::Float(f) => visitor.visit_f64(f),
            PhpToken::String(s) => visitor.visit_borrowed_bytes(s.as_bytes()),
            PhpToken::Array { elements } => visitor.visit_map(PhpMapAccess {
                de: self,
                remaining: elements,
            }),
            PhpToken::Object { properties, .. } => visitor.visit_map(PhpMapAccess {
                de: self,
                remaining: properties,
            }),
            PhpToken::Reference(r) => visitor.visit_i64(r),
            _ => Err(Error::from(ErrorKind::Deserialize {
                message: "Unexpected token".to_string(),
                position: Some(self.parser.position()),
            })),
        }
    }
}

impl<'de> Deserializer<'de> for &'_ mut PhpDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let token = self.parser.read_token()?;
        self.deserialize_token(visitor, token)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_i64() {
            Some(i) => visitor.visit_i64(i),
            None => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_f64() {
            Some(f) => visitor.visit_f64(f),
            None => self.deserialize_any(visitor),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_str().and_then(|x| x.to_str().ok()) {
            Some(s) => visitor.visit_borrowed_str(s),
            None => match self.parser.read_token()? {
                PhpToken::String(s) => {
                    let str_value = s.to_str()?;
                    visitor.visit_borrowed_str(str_value)
                }
                token => self.deserialize_token(visitor, token),
            },
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_str() {
            Some(s) => visitor.visit_borrowed_bytes(s.as_bytes()),
            None => self.deserialize_any(visitor),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let peeked = self
            .parser
            .peek_token()?
            .ok_or_else(|| Error::from(ErrorKind::Eof))?;
        if matches!(peeked, PhpTokenKind::Null) {
            let _ = self.parser.read_token()?;
            return visitor.visit_none();
        }

        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_seq_start() {
            Some(elements) => visitor.visit_seq(PhpSeqAccess {
                de: self,
                remaining: elements,
                next_index: 0,
            }),
            None => match self.parser.read_token()? {
                PhpToken::Array { elements } => visitor.visit_seq(PhpSeqAccess {
                    de: self,
                    remaining: elements,
                    next_index: 0,
                }),
                _ => Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected array".to_string(),
                    position: Some(self.parser.position()),
                })),
            },
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.next_token()? {
            Some(PhpToken::Array { elements }) if (elements as usize) == len => {
                visitor.visit_seq(PhpSeqAccess {
                    de: self,
                    remaining: elements,
                    next_index: 0,
                })
            }
            Some(PhpToken::Array { .. }) => Err(Error::from(ErrorKind::Deserialize {
                message: "Array length mismatch".to_string(),
                position: Some(self.parser.position()),
            })),
            _ => Err(Error::from(ErrorKind::Deserialize {
                message: "Expected array".to_string(),
                position: Some(self.parser.position()),
            })),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.read_token()? {
            PhpToken::Array { elements } => visitor.visit_map(PhpMapAccess {
                de: self,
                remaining: elements,
            }),
            PhpToken::Object { properties, .. } => visitor.visit_map(PhpMapAccess {
                de: self,
                remaining: properties,
            }),
            _ => Err(Error::from(ErrorKind::Deserialize {
                message: "Expected array or object".to_string(),
                position: Some(self.parser.position()),
            })),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        struct StringEnumAccess<'de> {
            variant: &'de str,
        }

        impl<'de> de::EnumAccess<'de> for StringEnumAccess<'de> {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: de::DeserializeSeed<'de>,
            {
                let value = seed.deserialize(
                    serde::de::value::BorrowedStrDeserializer::<Error>::new(self.variant),
                )?;
                Ok((value, self))
            }
        }

        impl<'de> de::VariantAccess<'de> for StringEnumAccess<'de> {
            type Error = Error;

            fn unit_variant(self) -> Result<(), Self::Error> {
                Ok(())
            }

            fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
            where
                T: de::DeserializeSeed<'de>,
            {
                Err(Error::from(ErrorKind::Deserialize {
                    message: "PHP string can only represent a unit enum variant".to_string(),
                    position: None,
                }))
            }

            fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                Err(Error::from(ErrorKind::Deserialize {
                    message: "PHP string can only represent a unit enum variant".to_string(),
                    position: None,
                }))
            }

            fn struct_variant<V>(
                self,
                _fields: &'static [&'static str],
                _visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                Err(Error::from(ErrorKind::Deserialize {
                    message: "PHP string can only represent a unit enum variant".to_string(),
                    position: None,
                }))
            }
        }

        struct MapEnumAccess<'a, 'de: 'a> {
            de: &'a mut PhpDeserializer<'de>,
        }

        impl<'de> de::EnumAccess<'de> for MapEnumAccess<'_, 'de> {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: de::DeserializeSeed<'de>,
            {
                let variant = seed.deserialize(&mut *self.de)?;
                Ok((variant, self))
            }
        }

        impl<'de> de::VariantAccess<'de> for MapEnumAccess<'_, 'de> {
            type Error = Error;

            fn unit_variant(self) -> Result<(), Self::Error> {
                match self.de.parser.read_token()? {
                    PhpToken::Null => {}
                    _ => {
                        return Err(Error::from(ErrorKind::Deserialize {
                            message: "Expected null payload for unit enum variant".to_string(),
                            position: Some(self.de.parser.position()),
                        }));
                    }
                }

                if self.de.parser.try_read_end() {
                    return Ok(());
                }

                Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected end of enum".to_string(),
                    position: Some(self.de.parser.position()),
                }))
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
            where
                T: de::DeserializeSeed<'de>,
            {
                let value = seed.deserialize(&mut *self.de)?;
                if self.de.parser.try_read_end() {
                    return Ok(value);
                }

                Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected end of enum".to_string(),
                    position: Some(self.de.parser.position()),
                }))
            }

            fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                let value = de::Deserializer::deserialize_seq(&mut *self.de, visitor)?;
                if self.de.parser.try_read_end() {
                    return Ok(value);
                }

                Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected end of enum".to_string(),
                    position: Some(self.de.parser.position()),
                }))
            }

            fn struct_variant<V>(
                self,
                _fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                let value = de::Deserializer::deserialize_map(&mut *self.de, visitor)?;
                if self.de.parser.try_read_end() {
                    return Ok(value);
                }

                Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected end of enum".to_string(),
                    position: Some(self.de.parser.position()),
                }))
            }
        }

        match self.parser.read_token()? {
            PhpToken::String(s) => visitor.visit_enum(StringEnumAccess {
                variant: s.to_str()?,
            }),
            PhpToken::Array { elements: 1 } => visitor.visit_enum(MapEnumAccess { de: self }),
            PhpToken::Object { properties: 1, .. } => {
                visitor.visit_enum(MapEnumAccess { de: self })
            }
            PhpToken::Array { .. } | PhpToken::Object { .. } => {
                Err(Error::from(ErrorKind::Deserialize {
                    message: "Expected single-entry map for enum".to_string(),
                    position: Some(self.parser.position()),
                }))
            }
            _ => Err(Error::from(ErrorKind::Deserialize {
                message: "Expected string or single-entry map for enum".to_string(),
                position: Some(self.parser.position()),
            })),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.try_read_str() {
            Some(s) => {
                let prop = s.to_property();
                visitor.visit_borrowed_bytes(prop.as_bytes())
            }
            None => {
                let token = self.parser.read_token()?;
                self.deserialize_token(visitor, token)
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct PhpSeqAccess<'a, 'de: 'a> {
    de: &'a mut PhpDeserializer<'de>,
    remaining: u32,
    next_index: i64,
}

impl<'de> SeqAccess<'de> for PhpSeqAccess<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            if self.de.parser.try_read_end() {
                return Ok(None);
            }
            if self.de.parser.peek_token()?.is_none() {
                return Err(Error::from(ErrorKind::Eof));
            }
            return Err(Error::from(ErrorKind::Deserialize {
                message: "Expected end of sequence".to_string(),
                position: Some(self.de.parser.position()),
            }));
        }

        let expected_index = self.next_index;
        match self.de.parser.try_read_i64() {
            Some(index) if index == expected_index => {}
            Some(index) => {
                return Err(Error::from(ErrorKind::Deserialize {
                    message: format!(
                        "Expected sequence index {expected_index}, found integer key {index}"
                    ),
                    position: Some(self.de.parser.position()),
                }));
            }
            None => match self.de.parser.read_token()? {
                PhpToken::Integer(index) if index == expected_index => {}
                PhpToken::Integer(index) => {
                    return Err(Error::from(ErrorKind::Deserialize {
                        message: format!(
                            "Expected sequence index {expected_index}, found integer key {index}"
                        ),
                        position: Some(self.de.parser.position()),
                    }));
                }
                _ => {
                    return Err(Error::from(ErrorKind::Deserialize {
                        message: format!(
                            "Expected sequence index {expected_index}, found non-integer key"
                        ),
                        position: Some(self.de.parser.position()),
                    }));
                }
            },
        }

        self.remaining -= 1;
        self.next_index += 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct PhpMapAccess<'a, 'de: 'a> {
    de: &'a mut PhpDeserializer<'de>,
    remaining: u32,
}

impl<'de> MapAccess<'de> for PhpMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            if self.de.parser.try_read_end() {
                return Ok(None);
            }
            return Err(Error::from(ErrorKind::Deserialize {
                message: "Expected end of map".to_string(),
                position: Some(self.de.parser.position()),
            }));
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhpBstr;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::{BTreeMap, HashMap},
        fmt,
        marker::PhantomData,
    };

    #[derive(Debug, PartialEq, Deserialize)]
    struct Person {
        name: String,
        age: i32,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
    pub enum CoPower {
        #[serde(rename = "N")]
        None,
        #[serde(rename = "Y")]
        Power,
        #[serde(rename = "S")]
        SuperPower,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct ObjectWrapper {
        inner: Person,
    }

    pub fn deserialize_vec_pair<'de, D, K, V>(deserializer: D) -> Result<Vec<(K, V)>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de>,
        V: Deserialize<'de>,
    {
        struct VecPairVisitor<K1, V1> {
            marker: PhantomData<Vec<(K1, V1)>>,
        }

        impl<'de, K1, V1> de::Visitor<'de> for VecPairVisitor<K1, V1>
        where
            K1: Deserialize<'de>,
            V1: Deserialize<'de>,
        {
            type Value = Vec<(K1, V1)>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map containing key value tuples")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some((key, value)) = map.next_entry()? {
                    values.push((key, value));
                }

                Ok(values)
            }
        }

        deserializer.deserialize_map(VecPairVisitor {
            marker: PhantomData,
        })
    }

    #[test]
    fn test_deserialize_null() {
        let input = b"N;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Option<String> = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_deserialize_boolean() {
        let input = b"b:1;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: bool = Deserialize::deserialize(&mut deserializer).unwrap();
        assert!(result);

        let input = b"b:0;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: bool = Deserialize::deserialize(&mut deserializer).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_deserialize_integer() {
        let input = b"i:123;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: i64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 123);
    }

    #[test]
    fn test_deserialize_large_integer() {
        let input = b"i:9223372036854775807;"; // i64::MAX
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: i64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn test_deserialize_negative_large_integer() {
        let input = b"i:-9223372036854775808;"; // i64::MIN
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: i64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, i64::MIN);
    }

    #[test]
    fn test_deserialize_float() {
        let input = b"d:3.33;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: f64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 3.33);
    }

    #[test]
    fn test_deserialize_float32() {
        let input = b"d:3.33;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: f32 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 3.33_f32);
    }

    #[test]
    fn test_deserialize_string() {
        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_char() {
        let input = b"s:1:\"x\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: char = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 'x');
    }

    #[test]
    fn test_deserialize_char_fast_path() {
        // Trailing serialized data keeps at least 16 bytes available so try_read_str() takes
        // its short-string fast path for the first token.
        let input = b"s:4:\"\xF0\x9F\x98\x80\";s:1:\"z\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: char = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, '😀');
    }

    #[test]
    fn test_deserialize_object() {
        let input = b"O:6:\"Person\":2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Person = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            result,
            Person {
                name: "Alice".to_string(),
                age: 30
            }
        );
    }

    #[test]
    fn test_deserialize_option_some() {
        let input = b"s:4:\"data\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Option<String> = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Some("data".to_string()));
    }

    #[test]
    fn test_deserialize_object_wrapper() {
        let input = b"O:13:\"ObjectWrapper\":1:{s:5:\"inner\";O:6:\"Person\":2:{s:4:\"name\";s:3:\"Bob\";s:3:\"age\";i:25;}}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: ObjectWrapper = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            result,
            ObjectWrapper {
                inner: Person {
                    name: "Bob".to_string(),
                    age: 25
                }
            }
        );
    }

    #[test]
    fn test_deserialize_assoc_array_to_tuple_vec() {
        // PHP: array("foo" => "bar", "baz" => "qux")
        let input = b"a:2:{s:3:\"foo\";s:3:\"bar\";s:3:\"baz\";s:3:\"qux\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Vec<(String, String)> = deserialize_vec_pair(&mut deserializer).unwrap();
        assert_eq!(
            result,
            vec![
                ("foo".to_string(), "bar".to_string()),
                ("baz".to_string(), "qux".to_string())
            ]
        );
    }

    #[test]
    fn test_deserialize_object_with_null() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct UserProfile {
            username: String,
            email: Option<String>,
            bio: Option<String>,
        }

        // PHP: new UserProfile(["username" => "john_doe", "email" => "john@example.com", "bio" => null])
        let input = b"O:11:\"UserProfile\":3:{s:8:\"username\";s:8:\"john_doe\";s:5:\"email\";s:16:\"john@example.com\";s:3:\"bio\";N;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: UserProfile = Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            result,
            UserProfile {
                username: "john_doe".to_string(),
                email: Some("john@example.com".to_string()),
                bio: None,
            }
        );
    }

    #[test]
    fn test_deserialize_mixed_key_array_to_tuple_vec() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(untagged)]
        enum Key {
            Int(i32),
            Str(String),
        }

        // PHP: array(0 => "value1", "key" => "value2", 1 => "value3")
        let input = b"a:3:{i:0;s:6:\"value1\";s:3:\"key\";s:6:\"value2\";i:1;s:6:\"value3\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);

        let result: Vec<(Key, String)> = deserialize_vec_pair(&mut deserializer).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (Key::Int(0), "value1".to_string()));
        assert_eq!(
            result[1],
            (Key::Str("key".to_string()), "value2".to_string())
        );
        assert_eq!(result[2], (Key::Int(1), "value3".to_string()));
    }

    #[test]
    fn test_deserialize_nested_assoc_array() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct NestedArray(
            #[serde(deserialize_with = "deserialize_vec_pair")] Vec<(String, String)>,
        );

        // PHP: array("outer" => array("inner" => "value"))
        let input = b"a:1:{s:5:\"outer\";a:1:{s:5:\"inner\";s:5:\"value\";}}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Vec<(String, NestedArray)> = deserialize_vec_pair(&mut deserializer).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "outer");
        assert_eq!(
            result[0].1,
            NestedArray(vec![("inner".to_string(), "value".to_string())])
        );
    }

    #[test]
    fn test_deserialize_vec_from_php_array() {
        let input = b"a:3:{i:0;s:2:\"aa\";i:1;s:2:\"bb\";i:2;s:2:\"cc\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Vec<String> = Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(result, vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn test_deserialize_vec_from_single_element_php_array() {
        let input = b"a:1:{i:0;i:10;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Vec<i64> = Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(result, vec![10]);
    }

    #[test]
    fn test_deserialize_vec_rejects_sparse_php_array() {
        let input = b"a:2:{i:1;s:2:\"aa\";i:3;s:2:\"bb\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Result<Vec<String>, _> = Deserialize::deserialize(&mut deserializer);
        let error = result.unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::Deserialize { message, .. }
                if message == "Expected sequence index 0, found integer key 1"
        ));
    }

    #[test]
    fn test_deserialize_vec_rejects_non_integer_php_array_key() {
        let input = b"a:1:{s:3:\"foo\";s:3:\"bar\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Result<Vec<String>, _> = Deserialize::deserialize(&mut deserializer);
        let error = result.unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::Deserialize { message, .. }
                if message == "Expected sequence index 0, found non-integer key"
        ));
    }

    #[test]
    fn test_deserialize_seq_explicitly() {
        #[derive(Debug, PartialEq)]
        struct ExplicitSeq(Vec<i64>);

        impl<'de> Deserialize<'de> for ExplicitSeq {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitSeqVisitor;

                impl<'de> de::Visitor<'de> for ExplicitSeqVisitor {
                    type Value = ExplicitSeq;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a sequence of integers")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        let mut values = Vec::new();
                        while let Some(value) = seq.next_element()? {
                            values.push(value);
                        }

                        Ok(ExplicitSeq(values))
                    }
                }

                deserializer.deserialize_seq(ExplicitSeqVisitor)
            }
        }

        let input = b"a:0:{}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result = ExplicitSeq::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, ExplicitSeq(vec![]));
    }

    #[test]
    fn test_deserialize_seq_missing_end_reports_eof() {
        let input = b"a:1:{i:0;i:10;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Result<Vec<i64>, _> = Deserialize::deserialize(&mut deserializer);
        let error = result.unwrap_err();

        assert!(matches!(error.kind(), ErrorKind::Eof));
    }

    #[test]
    fn test_deserialize_any_array_prefers_map() {
        #[derive(Debug, PartialEq)]
        struct AnyArray(Vec<(String, String)>);

        impl<'de> Deserialize<'de> for AnyArray {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct AnyArrayVisitor;

                impl<'de> de::Visitor<'de> for AnyArrayVisitor {
                    type Value = AnyArray;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a PHP array exposed as a map")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::MapAccess<'de>,
                    {
                        let mut entries = Vec::new();
                        while let Some((key, value)) = map.next_entry::<String, String>()? {
                            entries.push((key, value));
                        }

                        Ok(AnyArray(entries))
                    }
                }

                deserializer.deserialize_any(AnyArrayVisitor)
            }
        }

        let input = b"a:2:{s:3:\"foo\";s:3:\"bar\";s:3:\"baz\";s:3:\"qux\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result = AnyArray::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            result,
            AnyArray(vec![
                ("foo".to_string(), "bar".to_string()),
                ("baz".to_string(), "qux".to_string()),
            ])
        );
    }

    #[test]
    fn test_deserialize_assoc_array_to_hashmap() {
        // PHP: array("foo" => "bar", "baz" => "qux")
        let input = b"a:2:{s:3:\"foo\";s:3:\"bar\";s:3:\"baz\";s:3:\"qux\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: HashMap<String, String> = Deserialize::deserialize(&mut deserializer).unwrap();

        let mut expected = HashMap::new();
        expected.insert("foo".to_string(), "bar".to_string());
        expected.insert("baz".to_string(), "qux".to_string());

        assert_eq!(result, expected);
    }

    #[test]
    fn test_readme() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Example {
            name: String,
            age: i32,
            #[serde(rename = "isActive")]
            is_active: bool,
            scores: BTreeMap<u32, f64>,
            metadata: Metadata,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Metadata {
            id: i32,
            tags: BTreeMap<u32, String>,
        }

        let serialized = b"O:7:\"Example\":5:{s:4:\"name\";s:8:\"John Doe\";s:12:\"\0Example\0age\";i:42;s:11:\"\0*\0isActive\";b:1;s:6:\"scores\";a:3:{i:0;d:95.5;i:1;d:88.0;i:2;d:92.3;}s:8:\"metadata\";a:2:{s:2:\"id\";i:12345;s:4:\"tags\";a:3:{i:0;s:3:\"php\";i:1;s:4:\"rust\";i:2;s:13:\"serialization\";}}}";

        let mut deserializer = PhpDeserializer::new(serialized);
        let example = Example::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            example,
            Example {
                name: "John Doe".to_string(),
                age: 42,
                is_active: true,
                scores: BTreeMap::from([(0, 95.5), (1, 88.0), (2, 92.3),]),
                metadata: Metadata {
                    id: 12345,
                    tags: BTreeMap::from([
                        (0, "php".to_string()),
                        (1, "rust".to_string()),
                        (2, "serialization".to_string())
                    ]),
                }
            }
        );
    }

    #[test]
    fn test_deserialize_str_explicitly() {
        // Create a struct that explicitly calls deserialize_str
        #[derive(Debug, PartialEq)]
        struct ExplicitStr(String);

        impl<'de> Deserialize<'de> for ExplicitStr {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitStrVisitor;

                impl de::Visitor<'_> for ExplicitStrVisitor {
                    type Value = ExplicitStr;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(ExplicitStr(v.to_owned()))
                    }

                    fn visit_borrowed_bytes<E>(self, v: &'_ [u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        let s = std::str::from_utf8(v).map_err(|_| E::custom("Invalid UTF-8"))?;
                        Ok(ExplicitStr(s.to_owned()))
                    }
                }

                deserializer.deserialize_str(ExplicitStrVisitor)
            }
        }

        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: ExplicitStr = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, ExplicitStr("hello".to_string()));
    }

    #[test]
    fn test_deserialize_string_explicitly() {
        // Create a struct that explicitly calls deserialize_string
        #[derive(Debug, PartialEq)]
        struct ExplicitString(String);

        impl<'de> Deserialize<'de> for ExplicitString {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitStringVisitor;

                impl de::Visitor<'_> for ExplicitStringVisitor {
                    type Value = ExplicitString;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(ExplicitString(v.to_owned()))
                    }

                    fn visit_borrowed_bytes<E>(self, v: &'_ [u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        let s = std::str::from_utf8(v).map_err(|_| E::custom("Invalid UTF-8"))?;
                        Ok(ExplicitString(s.to_owned()))
                    }
                }

                deserializer.deserialize_string(ExplicitStringVisitor)
            }
        }

        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: ExplicitString = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, ExplicitString("hello".to_string()));
    }

    #[test]
    fn test_unicode_string() {
        #[derive(Debug, PartialEq)]
        struct ExplicitStr(String);

        impl<'de> Deserialize<'de> for ExplicitStr {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitStrVisitor;

                impl de::Visitor<'_> for ExplicitStrVisitor {
                    type Value = ExplicitStr;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(ExplicitStr(v.to_owned()))
                    }

                    fn visit_borrowed_bytes<E>(self, v: &'_ [u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        let s = std::str::from_utf8(v).map_err(|_| E::custom("Invalid UTF-8"))?;
                        Ok(ExplicitStr(s.to_owned()))
                    }
                }

                deserializer.deserialize_str(ExplicitStrVisitor)
            }
        }

        let input = b"s:8:\"\xF0\x9F\x91\x8B\xF0\x9F\x8C\x8D\";"; // "👋🌍"
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: ExplicitStr = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, ExplicitStr("👋🌍".to_string()));
    }

    #[test]
    fn test_invalid_utf8_string() {
        #[derive(Debug, PartialEq)]
        struct ExplicitStr(String);

        impl<'de> Deserialize<'de> for ExplicitStr {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitStrVisitor;

                impl de::Visitor<'_> for ExplicitStrVisitor {
                    type Value = ExplicitStr;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(ExplicitStr(v.to_owned()))
                    }
                }

                deserializer.deserialize_str(ExplicitStrVisitor)
            }
        }

        // Invalid UTF-8 sequence
        let input = b"s:4:\"\xFF\xFF\xFF\xFF\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result = ExplicitStr::deserialize(&mut deserializer);
        assert!(result.is_err(), "Should fail on invalid UTF-8");
    }

    #[test]
    fn test_deserialize_string_comparison_with_bytes() {
        // Create a struct that explicitly uses deserialize_bytes
        #[derive(Debug, PartialEq)]
        struct ExplicitBytes(Vec<u8>);

        impl<'de> Deserialize<'de> for ExplicitBytes {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ExplicitBytesVisitor;

                impl de::Visitor<'_> for ExplicitBytesVisitor {
                    type Value = ExplicitBytes;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("bytes")
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(ExplicitBytes(v.to_owned()))
                    }
                }

                deserializer.deserialize_bytes(ExplicitBytesVisitor)
            }
        }

        // This should work even with invalid UTF-8
        let input = b"s:4:\"\xFF\xFF\xFF\xFF\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result = ExplicitBytes::deserialize(&mut deserializer);
        assert!(
            result.is_ok(),
            "Should succeed on deserialize_bytes with invalid UTF-8"
        );
        assert_eq!(result.unwrap(), ExplicitBytes(vec![0xFF, 0xFF, 0xFF, 0xFF]));
    }

    #[test]
    fn test_deserialize_borrowed_str() {
        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: &str = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_borrowed_bytes_fallback() {
        // Test with invalid UTF-8
        let input = b"s:4:\"\xFF\xFF\xFF\xFF\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);

        let result: &[u8] = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, &[0xFF, 0xFF, 0xFF, 0xFF][..]);

        // Test with valid UTF-8 too
        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: &[u8] = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_deserialize_enum() {
        // Test deserializing "N" to CoPower::None
        let input = b"s:1:\"N\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: CoPower = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, CoPower::None);

        // Test deserializing "Y" to CoPower::Power
        let input = b"s:1:\"Y\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: CoPower = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, CoPower::Power);

        // Test deserializing "S" to CoPower::SuperPower
        let input = b"s:1:\"S\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: CoPower = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, CoPower::SuperPower);
    }

    #[test]
    fn test_deserialize_enum_in_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Character {
            name: String,
            power: CoPower,
        }

        // PHP: array("name" => "Superman", "power" => "S")
        let input = b"a:2:{s:4:\"name\";s:8:\"Superman\";s:5:\"power\";s:1:\"S\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Character = Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            result,
            Character {
                name: "Superman".to_string(),
                power: CoPower::SuperPower,
            }
        );
    }

    #[test]
    fn test_deserialize_external_tagged_enum() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(tag = "type", content = "value")]
        enum Message {
            Text(String),
            Number(i32),
        }

        // PHP: array("type" => "Text", "value" => "Hello")
        let input = b"a:2:{s:4:\"type\";s:4:\"Text\";s:5:\"value\";s:5:\"Hello\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Message::Text("Hello".to_string()));

        // PHP: array("type" => "Number", "value" => 42)
        let input = b"a:2:{s:4:\"type\";s:6:\"Number\";s:5:\"value\";i:42;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Message::Number(42));
    }

    #[test]
    fn test_deserialize_externally_tagged_newtype_enum() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Message {
            Text(String),
            Number(i32),
        }

        let input = b"a:1:{s:4:\"Text\";s:5:\"Hello\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Message::Text("Hello".to_string()));

        let input = b"a:1:{s:6:\"Number\";i:42;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Message::Number(42));
    }

    #[test]
    fn test_deserialize_externally_tagged_tuple_enum() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Shape {
            Point(i32, i32),
        }

        let input = b"a:1:{s:5:\"Point\";a:2:{i:0;i:3;i:1;i:4;}}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Shape = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Shape::Point(3, 4));
    }

    #[test]
    fn test_deserialize_externally_tagged_struct_enum() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Message {
            User { name: String, age: i32 },
        }

        let input = b"a:1:{s:4:\"User\";a:2:{s:4:\"name\";s:3:\"Bob\";s:3:\"age\";i:30;}}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            result,
            Message::User {
                name: "Bob".to_string(),
                age: 30,
            }
        );
    }

    #[test]
    fn test_deserialize_externally_tagged_enum_rejects_multi_entry_wrapper() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Message {
            Text(String),
        }

        let input = b"a:2:{s:4:\"Text\";s:5:\"Hello\";s:5:\"extra\";s:3:\"bad\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Result<Message, _> = Deserialize::deserialize(&mut deserializer);
        let error = result.unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::Deserialize { message, .. }
                if message == "Expected single-entry map for enum"
        ));
    }

    #[test]
    fn test_deserialize_externally_tagged_unit_enum_map_form() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Direction {
            North,
            South,
        }

        // Unit variant encoded as a single-entry map with a null value
        let input = b"a:1:{s:5:\"North\";N;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Direction = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Direction::North);

        let input = b"a:1:{s:5:\"South\";N;}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Direction = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Direction::South);
    }

    #[test]
    fn test_deserialize_externally_tagged_enum_object_form() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Message {
            Text(String),
        }

        // Externally tagged enum encoded as a PHP object with one property
        let input = b"O:7:\"MyClass\":1:{s:4:\"Text\";s:5:\"Hello\";}";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: Message = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Message::Text("Hello".to_string()));
    }

    #[test]
    fn test_from_parser_and_into_parser() {
        // Create a parser with a complex structure
        let input = b"i:42;s:5:\"hello\";b:1;";
        let mut parser = PhpParser::new(input);

        // Parse the first token manually
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(42));

        // Create a deserializer from the parser at its current position
        let mut deserializer = PhpDeserializer::from_parser(parser);

        // Deserialize the next item (a string)
        let result: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, "hello");

        // Get the parser back
        let mut parser = deserializer.into_parser();

        // Continue parsing with the parser
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Boolean(true));

        // Verify no more tokens
        assert!(parser.next_token().unwrap().is_none());
    }

    #[test]
    fn test_partial_deserialization() {
        // Create a data structure with multiple values
        let input = b"a:3:{i:0;s:5:\"first\";i:1;s:6:\"second\";i:2;s:5:\"third\";}";
        let mut parser = PhpParser::new(input);

        // Skip the array opening token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Array { elements: 3 });

        // Skip the first key
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(0));

        // Create a deserializer from the parser at current position
        let mut deserializer = PhpDeserializer::from_parser(parser);

        // Deserialize the first value
        let first_value: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(first_value, "first");

        // Get the parser back
        let mut parser = deserializer.into_parser();

        // Skip the second key
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(1));

        // Create another deserializer
        let mut deserializer = PhpDeserializer::from_parser(parser);

        // Deserialize the second value
        let second_value: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(second_value, "second");

        // Get the parser back again
        let mut parser = deserializer.into_parser();

        // Continue with manual parsing for the rest
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(2));

        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::String(PhpBstr::new(b"third")));

        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::End);

        // Verify no more tokens
        assert!(parser.next_token().unwrap().is_none());
    }

    #[test]
    fn test_nested_object_with_from_parser() {
        // Create a nested object structure
        let input = b"O:7:\"Example\":1:{s:4:\"user\";O:4:\"User\":2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}}";
        let mut parser = PhpParser::new(input);

        // Skip to the 'user' object
        parser.next_token().unwrap(); // Skip outer object
        parser.next_token().unwrap(); // Skip "user" string

        // Create a deserializer from the parser at user object position
        let mut deserializer = PhpDeserializer::from_parser(parser);

        // Deserialize just the User object
        let user: Person = Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(
            user,
            Person {
                name: "Alice".to_string(),
                age: 30
            }
        );

        // Get the parser back
        let mut parser = deserializer.into_parser();
        assert_eq!(parser.read_token().unwrap(), PhpToken::End);

        // Verify we're at the end
        assert_eq!(parser.position(), input.len());
    }
}
