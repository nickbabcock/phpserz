use crate::errors::{Error, ErrorKind};
use crate::parser::{PhpParser, PhpToken, PhpTokenKind};
use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess};

pub struct PhpDeserializer<'de> {
    parser: PhpParser<'de>,
}

impl<'de> PhpDeserializer<'de> {
    #[must_use]
    pub const fn new(data: &'de [u8]) -> Self {
        PhpDeserializer {
            parser: PhpParser::new(data),
        }
    }
}

impl<'de> Deserializer<'de> for &'_ mut PhpDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.read_token()? {
            PhpToken::Null => visitor.visit_unit(),
            PhpToken::Boolean(b) => visitor.visit_bool(b),
            PhpToken::Integer(i) => visitor.visit_i32(i),
            PhpToken::Float(f) => visitor.visit_f64(f),
            PhpToken::String(s) => visitor.visit_borrowed_bytes(s.as_bytes()),
            PhpToken::Array { .. } => visitor.visit_seq(self),
            PhpToken::Object { .. } => visitor.visit_map(self),
            PhpToken::Reference(r) => visitor.visit_i32(r),
            PhpToken::End => Err(Error::from(ErrorKind::Deserialize(
                "Unexpected end token".to_string(),
            ))),
        }
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
        let peeked = self
            .parser
            .peek_token()?
            .ok_or_else(|| Error::from(ErrorKind::Eof))?;
        if matches!(peeked, PhpTokenKind::Null) {
            self.parser.consume_lookahead();
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
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.parser.next_token()? {
            Some(PhpToken::Array { elements }) if (elements as usize) == len => {
                visitor.visit_seq(self)
            }
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
        match self.parser.read_token()? {
            PhpToken::Array { .. } | PhpToken::Object { .. } => visitor.visit_map(self),
            _ => Err(Error::from(ErrorKind::Deserialize(
                "Expected array or object".to_string(),
            ))),
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
        match self.parser.read_token()? {
            PhpToken::Null => visitor.visit_unit(),
            PhpToken::Boolean(b) => visitor.visit_bool(b),
            PhpToken::Integer(i) => visitor.visit_i32(i),
            PhpToken::Float(f) => visitor.visit_f64(f),
            PhpToken::String(s) => {
                let (name, _) = s.to_property()?;
                visitor.visit_borrowed_str(name)
            }
            PhpToken::Array { .. } => visitor.visit_seq(self),
            PhpToken::Object { .. } => visitor.visit_map(self),
            PhpToken::Reference(r) => visitor.visit_i32(r),
            PhpToken::End => Err(Error::from(ErrorKind::Deserialize(
                "Unexpected end token".to_string(),
            ))),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de> SeqAccess<'de> for &'_ mut PhpDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let peeked = self
            .parser
            .peek_token()?
            .ok_or_else(|| Error::from(ErrorKind::Eof))?;
        if matches!(peeked, PhpTokenKind::End) {
            self.parser.consume_lookahead();
            return Ok(None);
        }

        seed.deserialize(&mut **self).map(Some)
    }
}

#[derive(Debug)]
enum MapState {
    Key,
    Value,
}

impl<'de> MapAccess<'de> for &'_ mut PhpDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let peeked = self
            .parser
            .peek_token()?
            .ok_or_else(|| Error::from(ErrorKind::Eof))?;
        if matches!(peeked, PhpTokenKind::End) {
            self.parser.consume_lookahead();
            return Ok(None);
        }

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
    use std::{collections::HashMap, fmt, marker::PhantomData};

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
        let result: i32 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 123);
    }

    #[test]
    fn test_deserialize_float() {
        let input = b"d:3.14;";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: f64 = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, 3.14);
    }

    #[test]
    fn test_deserialize_string() {
        let input = b"s:5:\"hello\";";
        let mut deserializer = PhpDeserializer::new(&input[..]);
        let result: String = Deserialize::deserialize(&mut deserializer).unwrap();
        assert_eq!(result, "hello");
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
}
