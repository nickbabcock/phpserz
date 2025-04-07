use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess};
use std::io::BufRead;

use crate::errors::{Error, ErrorKind};
use crate::parser::{PhpParser, PhpToken, PhpTokenKind};

pub struct PhpDeserializer<R> {
    parser: PhpParser<R>,
    buffer: Vec<u8>,
}

impl<R: BufRead> PhpDeserializer<R> {
    pub fn new(reader: R) -> Self {
        PhpDeserializer {
            parser: PhpParser::new(reader),
            buffer: Vec::new(),
        }
    }
}

impl<'de, 'a, R: BufRead> Deserializer<'de> for &'_ mut PhpDeserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let result = match self.parser.next_token(&mut self.buffer)? {
            Some(PhpToken::Null) => visitor.visit_unit(),
            Some(PhpToken::Boolean(b)) => visitor.visit_bool(b),
            Some(PhpToken::Integer(i)) => visitor.visit_i32(i),
            Some(PhpToken::Float(f)) => visitor.visit_f64(f),
            Some(PhpToken::String(s)) => visitor.visit_str(s.as_str()),
            Some(PhpToken::Array { .. }) => visitor.visit_seq(self),
            Some(PhpToken::Object { .. }) => visitor.visit_map(self),
            Some(PhpToken::Reference(r)) => visitor.visit_i32(r),
            Some(PhpToken::End) => Err(Error::from(ErrorKind::Deserialize(
                "Unexpected end token".to_string(),
            ))),
            None => Err(Error::from(ErrorKind::Deserialize(
                "Unexpected end of input".to_string(),
            ))),
        };
        result
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
        self.deserialize_any(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
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
        self.deserialize_any(visitor)
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
        match self.parser.peek_token()? {
            Some(PhpTokenKind::Null) => {
                self.parser.consume_lookahead();
                visitor.visit_none()
            }
            Some(_) => visitor.visit_some(self),
            None => Err(Error::from(ErrorKind::Deserialize(
                "Unexpected end of input".to_string(),
            ))),
        }
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
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.next_token(&mut self.buffer)? {
            Some(PhpToken::Array { elements }) if elements == len => visitor.visit_seq(self),
            Some(PhpToken::Array { .. }) => Err(Error::from(ErrorKind::Deserialize(
                "Array length mismatch".to_string(),
            ))),
            _ => Err(Error::from(ErrorKind::Deserialize(
                "Expected array".to_string(),
            ))),
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
        let res = match self.parser.next_token(&mut self.buffer)? {
            Some(PhpToken::Array { .. }) => visitor.visit_map(self),
            Some(PhpToken::Object { .. }) => visitor.visit_map(self),
            _ => Err(Error::from(ErrorKind::Deserialize(
                "Expected array or object".to_string(),
            ))),
        };

        res
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
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::from(ErrorKind::Deserialize(
            "PHP does not support enum deserialization".to_string(),
        )))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, 'a, R: BufRead> SeqAccess<'de> for &'_ mut PhpDeserializer<R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.parser.peek_token()? {
            Some(PhpTokenKind::End) => {
                self.parser.consume_lookahead();
                return Ok(None);
            }
            Some(_) => {}
            None => {
                return Err(Error::from(ErrorKind::Deserialize(
                    "Unexpected end of input".to_string(),
                )));
            }
        };

        seed.deserialize(&mut **self).map(Some)
    }
}

#[derive(Debug)]
enum MapState {
    Key,
    Value,
}

impl<'de, 'a, R: BufRead> MapAccess<'de> for &'_ mut PhpDeserializer<R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.parser.peek_token()? {
            Some(PhpTokenKind::End) => {
                self.parser.consume_lookahead();
                return Ok(None);
            }
            Some(_) => {}
            None => {
                return Err(Error::from(ErrorKind::Deserialize(
                    "Unexpected end of input".to_string(),
                )));
            }
        };

        seed.deserialize(&mut **self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut **self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::io::Cursor;

    #[derive(Debug, PartialEq, Deserialize)]
    struct Person {
        name: String,
        age: i32,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Data {
        value: Option<String>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct NestedArray {
        data: Vec<Vec<i32>>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct ObjectWrapper {
        inner: Person,
    }

    #[test]
    fn test_deserialize_null() {
        let input = b"N;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: Option<String> = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_deserialize_boolean() {
        let input = b"b:1;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: bool = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, true);

        let input = b"b:0;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: bool = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_deserialize_integer() {
        let input = b"i:123;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: i32 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 123);
    }

    #[test]
    fn test_deserialize_float() {
        let input = b"d:3.14;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: f64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 3.14);
    }

    #[test]
    fn test_deserialize_string() {
        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_array() {
        let input = b"a:3:{i:0;i:1;i:1;i:2;i:2;i:3;}";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: Vec<i32> = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_deserialize_nested_array() {
        let input = b"a:2:{i:0;a:2:{i:0;i:1;i:1;i:2;}i:1;a:1:{i:0;i:3;}}";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: NestedArray = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            result,
            NestedArray {
                data: vec![vec![1, 2], vec![3]]
            }
        );
    }

    #[test]
    fn test_deserialize_object() {
        let input = b"O:6:\"Person\":2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
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
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: Option<String> = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, Some("data".to_string()));
    }

    #[test]
    fn test_deserialize_struct_with_option() {
        let input = b"a:1:{s:5:\"value\";s:4:\"test\";}";
        #[derive(Debug, PartialEq, Deserialize)]
        struct TestStruct {
            value: Option<String>,
        }
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: TestStruct = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            result,
            TestStruct {
                value: Some("test".to_string())
            }
        );

        let input = b"a:1:{s:5:\"value\";N;}";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: TestStruct = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, TestStruct { value: None });
    }

    #[test]
    fn test_deserialize_object_wrapper() {
        let input = b"O:13:\"ObjectWrapper\":1:{s:5:\"inner\";O:6:\"Person\":2:{s:4:\"name\";s:3:\"Bob\";s:3:\"age\";i:25;}}";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
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
    fn test_deserialize_reference() {
        let input = b"a:1:{i:0;s:5:\"hello\";}r:2;";
        let mut deserializer = PhpDeserializer::new(Cursor::new(input));
        let result: i32 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 2);
    }
}
