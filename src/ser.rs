use crate::errors::{Error, ErrorKind};
use serde::ser::{self, Impossible, Serialize};
use std::io::Write;

/// How named Rust structs are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StructStyle {
    /// Encode structs as PHP associative arrays (`a:N:{s:"field";value;...}`).
    #[default]
    Array,

    /// Encode structs as PHP objects (`O:len:"Name":N:{s:"field";value;...}`).
    ///
    /// The class name is the serde struct name.
    Object,
}

/// A serializer for the PHP serialization format.
///
/// Produces bytes that can be read back by [`crate::PhpDeserializer`], allowing
/// `T -> PHP bytes -> T` roundtrips for ordinary serde data.
#[derive(Debug)]
pub struct PhpSerializer<W> {
    writer: W,
    struct_style: StructStyle,
}

impl<W: Write> PhpSerializer<W> {
    /// Create a new serializer that writes to the given writer.
    #[must_use]
    pub const fn new(writer: W) -> Self {
        PhpSerializer {
            writer,
            struct_style: StructStyle::Array,
        }
    }

    /// Set how named structs are encoded (default [`StructStyle::Array`]).
    #[must_use]
    pub const fn struct_style(mut self, style: StructStyle) -> Self {
        self.struct_style = style;
        self
    }

    /// Consume this serializer and return the underlying writer.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.writer.write_all(bytes)?;
        Ok(())
    }

    /// Write a PHP string token: `s:LEN:"BYTES";`.
    fn write_str_token(&mut self, bytes: &[u8]) -> Result<(), Error> {
        write!(self.writer, "s:{}:\"", bytes.len())?;
        self.writer.write_all(bytes)?;
        self.writer.write_all(b"\";")?;
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<(), Error> {
        write!(self.writer, "i:{value};")?;
        Ok(())
    }
}

fn float_to_token<W: Write>(writer: &mut W, value: f64) -> Result<(), Error> {
    if !value.is_finite() {
        return Err(Error::from(ErrorKind::Serialize {
            message: "cannot serialize non-finite float".to_string(),
        }));
    }
    write!(writer, "d:{value};")?;
    Ok(())
}

impl<'a, W: Write> ser::Serializer for &'a mut PhpSerializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, W>;
    type SerializeTuple = SeqSerializer<'a, W>;
    type SerializeTupleStruct = SeqSerializer<'a, W>;
    type SerializeTupleVariant = TupleVariantSerializer<'a, W>;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = StructSerializer<'a, W>;
    type SerializeStructVariant = StructVariantSerializer<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.write_bytes(if v { b"b:1;" } else { b"b:0;" })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.write_i64(v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.write_i64(value)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.write_i64(i64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.write_i64(value)
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.write_i64(value)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        // Format the f32 directly to preserve its shortest roundtrip representation.
        if !v.is_finite() {
            return Err(Error::from(ErrorKind::Serialize {
                message: "cannot serialize non-finite float".to_string(),
            }));
        }
        write!(self.writer, "d:{v};")?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        float_to_token(&mut self.writer, v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        self.write_str_token(v.encode_utf8(&mut buf).as_bytes())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_str_token(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_str_token(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.write_bytes(b"N;")
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.write_bytes(b"N;")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.write_bytes(b"N;")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.write_str_token(variant.as_bytes())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.write_bytes(b"a:1:{")?;
        self.write_str_token(variant.as_bytes())?;
        value.serialize(&mut *self)?;
        self.write_bytes(b"}")
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let len = len.ok_or_else(unknown_len_error)?;
        write!(self.writer, "a:{len}:{{")?;
        Ok(SeqSerializer {
            ser: self,
            next_index: 0,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.write_bytes(b"a:1:{")?;
        self.write_str_token(variant.as_bytes())?;
        write!(self.writer, "a:{len}:{{")?;
        Ok(TupleVariantSerializer {
            ser: self,
            next_index: 0,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let len = len.ok_or_else(unknown_len_error)?;
        write!(self.writer, "a:{len}:{{")?;
        Ok(MapSerializer { ser: self })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        match self.struct_style {
            StructStyle::Array => write!(self.writer, "a:{len}:{{")?,
            StructStyle::Object => {
                write!(self.writer, "O:{}:\"{}\":{}:{{", name.len(), name, len)?;
            }
        }
        Ok(StructSerializer { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.write_bytes(b"a:1:{")?;
        self.write_str_token(variant.as_bytes())?;
        write!(self.writer, "a:{len}:{{")?;
        Ok(StructVariantSerializer { ser: self })
    }
}

fn int_range_error() -> Error {
    Error::from(ErrorKind::Serialize {
        message: "integer out of range for PHP i64".to_string(),
    })
}

fn unknown_len_error() -> Error {
    Error::from(ErrorKind::Serialize {
        message: "PHP serialization requires a known sequence/map length".to_string(),
    })
}

/// Serializes sequences, tuples, and tuple structs as int-keyed PHP arrays.
pub struct SeqSerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
    next_index: i64,
}

impl<W: Write> ser::SerializeSeq for SeqSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.ser.write_i64(self.next_index)?;
        self.next_index += 1;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_bytes(b"}")
    }
}

impl<W: Write> ser::SerializeTuple for SeqSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<W: Write> ser::SerializeTupleStruct for SeqSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializes tuple enum variants as `a:1:{s:"Variant";a:N:{i:0;...};}`.
pub struct TupleVariantSerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
    next_index: i64,
}

impl<W: Write> ser::SerializeTupleVariant for TupleVariantSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.ser.write_i64(self.next_index)?;
        self.next_index += 1;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_bytes(b"}}")
    }
}

/// Serializes maps as PHP associative arrays; keys must be integers or strings.
pub struct MapSerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
}

impl<W: Write> ser::SerializeMap for MapSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(MapKeySerializer { ser: self.ser })
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_bytes(b"}")
    }
}

/// Serializes structs as PHP arrays or objects, depending on [`StructStyle`].
pub struct StructSerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
}

impl<W: Write> ser::SerializeStruct for StructSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.ser.write_str_token(key.as_bytes())?;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_bytes(b"}")
    }
}

/// Serializes struct enum variants as `a:1:{s:"Variant";a:N:{s:"field";...};}`.
pub struct StructVariantSerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
}

impl<W: Write> ser::SerializeStructVariant for StructVariantSerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.ser.write_str_token(key.as_bytes())?;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.write_bytes(b"}}")
    }
}

/// A serializer used for PHP array keys, which may only be integers or strings.
struct MapKeySerializer<'a, W> {
    ser: &'a mut PhpSerializer<W>,
}

impl<W: Write> MapKeySerializer<'_, W> {
    fn unsupported(self) -> Error {
        Error::from(ErrorKind::Serialize {
            message: "PHP array keys must be integers or strings".to_string(),
        })
    }
}

impl<W: Write> ser::Serializer for MapKeySerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.ser.write_i64(value)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.ser.write_i64(i64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.ser.write_i64(value)
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        let value = i64::try_from(v).map_err(|_| int_range_error())?;
        self.ser.write_i64(value)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.ser.write_str_token(v.as_bytes())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        self.ser.write_str_token(v.encode_utf8(&mut buf).as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.ser.write_str_token(v)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        // `Option` is a transparent wrapper, like a newtype struct: the inner type
        // decides whether the key is a valid PHP int/string key. (`None` keys are
        // rejected by `serialize_none`, since PHP arrays have no null key.)
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(self.unsupported())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(self.unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhpDeserializer;
    use rstest::rstest;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
        let mut serializer = PhpSerializer::new(Vec::new());
        value.serialize(&mut serializer)?;
        Ok(serializer.into_inner())
    }

    fn to_string<T: Serialize>(value: &T) -> String {
        String::from_utf8(to_vec(value).unwrap()).unwrap()
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Person {
        name: String,
        age: i32,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ObjectWrapper {
        inner: Person,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
    enum CoPower {
        #[serde(rename = "N")]
        None,
        #[serde(rename = "Y")]
        Power,
        #[serde(rename = "S")]
        SuperPower,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Shape {
        Unit,
        Newtype(i32),
        Tuple(i32, i32),
        Struct { x: i32, y: i32 },
    }

    #[rstest]
    #[case(true, "b:1;")]
    #[case(false, "b:0;")]
    fn test_serialize_bool(#[case] value: bool, #[case] expected: &str) {
        assert_eq!(to_string(&value), expected);
    }

    #[rstest]
    #[case(0_i64, "i:0;")]
    #[case(123_i64, "i:123;")]
    #[case(-7_i64, "i:-7;")]
    #[case(i64::MAX, "i:9223372036854775807;")]
    #[case(i64::MIN, "i:-9223372036854775808;")]
    fn test_serialize_integer(#[case] value: i64, #[case] expected: &str) {
        assert_eq!(to_string(&value), expected);
    }

    #[test]
    fn test_serialize_float() {
        assert_eq!(to_string(&3.33_f64), "d:3.33;");
        assert_eq!(to_string(&3.33_f32), "d:3.33;");
    }

    #[test]
    fn test_serialize_string_byte_length() {
        // Each emoji is 4 bytes, so the length is 8 even though it is 2 chars.
        assert_eq!(to_string(&"👋🌍"), "s:8:\"👋🌍\";");
        assert_eq!(to_string(&"hello"), "s:5:\"hello\";");
    }

    #[test]
    fn test_serialize_char() {
        assert_eq!(to_string(&'a'), "s:1:\"a\";");
    }

    #[test]
    fn test_serialize_option() {
        assert_eq!(to_string(&Option::<i32>::None), "N;");
        assert_eq!(to_string(&Some(5_i32)), "i:5;");
    }

    #[test]
    fn test_serialize_unit() {
        assert_eq!(to_string(&()), "N;");
    }

    #[test]
    fn test_serialize_seq() {
        assert_eq!(
            to_string(&vec![1_i32, 2, 3]),
            "a:3:{i:0;i:1;i:1;i:2;i:2;i:3;}"
        );
    }

    #[test]
    fn test_serialize_tuple() {
        assert_eq!(to_string(&(1_i32, "x")), "a:2:{i:0;i:1;i:1;s:1:\"x\";}");
    }

    #[test]
    fn test_serialize_map() {
        let mut map = BTreeMap::new();
        map.insert("a", 1_i32);
        map.insert("b", 2);
        assert_eq!(to_string(&map), "a:2:{s:1:\"a\";i:1;s:1:\"b\";i:2;}");
    }

    #[test]
    fn test_serialize_struct_array_default() {
        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };
        assert_eq!(
            to_string(&person),
            "a:2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}"
        );
    }

    #[test]
    fn test_serialize_struct_object_style() {
        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };
        let mut serializer = PhpSerializer::new(Vec::new()).struct_style(StructStyle::Object);
        person.serialize(&mut serializer).unwrap();
        assert_eq!(
            String::from_utf8(serializer.into_inner()).unwrap(),
            "O:6:\"Person\":2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}"
        );
    }

    #[test]
    fn test_serialize_enum_variants() {
        assert_eq!(to_string(&CoPower::SuperPower), "s:1:\"S\";");
        assert_eq!(to_string(&Shape::Unit), "s:4:\"Unit\";");
        assert_eq!(to_string(&Shape::Newtype(7)), "a:1:{s:7:\"Newtype\";i:7;}");
        assert_eq!(
            to_string(&Shape::Tuple(1, 2)),
            "a:1:{s:5:\"Tuple\";a:2:{i:0;i:1;i:1;i:2;}}"
        );
        assert_eq!(
            to_string(&Shape::Struct { x: 1, y: 2 }),
            "a:1:{s:6:\"Struct\";a:2:{s:1:\"x\";i:1;s:1:\"y\";i:2;}}"
        );
    }

    fn roundtrip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let bytes = to_vec(value).unwrap();
        let mut de = PhpDeserializer::new(&bytes);
        let back = T::deserialize(&mut de).unwrap();
        assert_eq!(&back, value);
    }

    #[test]
    fn test_roundtrip_scalars() {
        roundtrip(&true);
        roundtrip(&42_i64);
        roundtrip(&(-1_i32));
        roundtrip(&3.33_f64);
        roundtrip(&"hello".to_string());
        roundtrip(&Option::<i32>::None);
        roundtrip(&Some(9_i32));
    }

    #[test]
    fn test_roundtrip_struct_both_styles() {
        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };
        roundtrip(&person);

        // Object style also roundtrips through the deserializer.
        let mut serializer = PhpSerializer::new(Vec::new()).struct_style(StructStyle::Object);
        person.serialize(&mut serializer).unwrap();
        let bytes = serializer.into_inner();
        let mut de = PhpDeserializer::new(&bytes);
        let back = Person::deserialize(&mut de).unwrap();
        assert_eq!(back, person);
    }

    #[test]
    fn test_roundtrip_nested_and_collections() {
        roundtrip(&ObjectWrapper {
            inner: Person {
                name: "Bob".to_string(),
                age: 25,
            },
        });
        roundtrip(&vec![1_i32, 2, 3]);

        let mut map = BTreeMap::new();
        map.insert("a".to_string(), 1_i32);
        map.insert("b".to_string(), 2);
        roundtrip(&map);
    }

    #[test]
    fn test_roundtrip_enums() {
        roundtrip(&CoPower::None);
        roundtrip(&CoPower::Power);
        roundtrip(&Shape::Unit);
        roundtrip(&Shape::Newtype(7));
        roundtrip(&Shape::Tuple(1, 2));
        roundtrip(&Shape::Struct { x: 1, y: 2 });
    }

    #[test]
    fn test_error_unsigned_overflow() {
        let err = to_vec(&u64::MAX).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Serialize { .. }));
    }

    #[test]
    fn test_error_non_finite_float() {
        let err = to_vec(&f64::NAN).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Serialize { .. }));
        let err = to_vec(&f64::INFINITY).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Serialize { .. }));
    }

    #[test]
    fn test_error_unsupported_map_key() {
        let mut map = BTreeMap::new();
        map.insert(true, 1_i32);
        let err = to_vec(&map).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Serialize { .. }));
    }

    #[test]
    fn test_optional_map_key() {
        // `Some(valid_key)` is transparent and roundtrips as a plain string key.
        let mut map = BTreeMap::new();
        map.insert(Some("a".to_string()), 1_i32);
        map.insert(Some("b".to_string()), 2);
        assert_eq!(to_string(&map), "a:2:{s:1:\"a\";i:1;s:1:\"b\";i:2;}");
        roundtrip(&map);

        // A `None` key is rejected: PHP arrays have no null key.
        let mut none_key = BTreeMap::new();
        none_key.insert(Option::<String>::None, 1_i32);
        let err = to_vec(&none_key).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Serialize { .. }));
    }
}
