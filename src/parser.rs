use std::io::BufRead;

use crate::errors::{Error, ErrorKind, PhpParseError};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct PhpStr<'a> {
    data: &'a str,
    visibility: PhpVisibility,
}

impl<'a> PhpStr<'a> {
    pub fn new(data: &'a str, visibility: PhpVisibility) -> Self {
        Self { data, visibility }
    }

    pub(crate) fn public(data: &'a str) -> Self {
        Self::new(data, PhpVisibility::Public)
    }

    pub(crate) fn protected(data: &'a str) -> Self {
        Self::new(data, PhpVisibility::Protected)
    }

    pub(crate) fn private(data: &'a str) -> Self {
        Self::new(data, PhpVisibility::Private)
    }

    pub fn visibility(&self) -> PhpVisibility {
        self.visibility
    }

    pub fn as_str(&self) -> &str {
        self.data
    }
}

impl AsRef<str> for PhpStr<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum PhpVisibility {
    Public,
    Protected,
    Private,
}

#[derive(Debug, PartialEq)]
pub enum PhpToken<'a> {
    Null,
    Boolean(bool),
    Integer(i32),
    Float(f64),
    String(PhpStr<'a>),
    Array { elements: usize },
    Object { class: PhpStr<'a>, properties: u32 },
    End,
    Reference(i32),
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum PhpTokenKind {
    Null,
    Boolean,
    Integer,
    Float,
    String,
    Array,
    Object,
    End,
    Reference,
}

pub struct PhpParser<R> {
    reader: R,
    lookahead: Option<(PhpTokenKind, usize)>,
    position: usize,
}

impl<R: BufRead> PhpParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            lookahead: None,
            position: 0,
        }
    }

    fn create_error(&self, message: &str) -> Error {
        ErrorKind::PhpParse(PhpParseError {
            message: message.to_string(),
            position: self.position,
        })
        .into()
    }

    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        self.position += 1;

        if buf[0] != expected {
            return Err(self.create_error(&format!(
                "Expected byte '{}', found '{}'",
                expected as char, buf[0] as char
            )));
        }

        Ok(())
    }

    fn read_str(&mut self, storage: &mut Vec<u8>) -> Result<(usize, usize, PhpVisibility), Error> {
        storage.clear();

        let start_position = self.position;

        // Read the string length
        let bytes_read = self.reader.read_until(b':', storage)?;
        self.position += bytes_read;

        if !matches!(storage.pop(), Some(b':')) {
            return Err(self.create_error("Expected ':' after string length"));
        }

        let s = std::str::from_utf8(storage)
            .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

        let length = s
            .parse::<usize>()
            .map_err(|e| Error::from(ErrorKind::ParseInt(e, start_position)))?;

        // Read the string, which is enclosed in double quotes
        storage.resize(length + 2, 0);
        self.reader.read_exact(storage)?;
        self.position += storage.len();

        if !matches!(storage.as_slice(), [b'"', .., b'"']) {
            return Err(self.create_error("String is not properly enclosed in double quotes"));
        }

        match storage.as_slice() {
            [b'"', 0, b'*', 0, ..] => Ok((4, length - 3, PhpVisibility::Protected)), 
            [b'"', 0, tail @ ..] => {
                let mut tail = tail;
                while let Some((c, rest)) = tail.split_first() {
                    if *c == b'\0' {
                        return Ok((storage.len() - rest.len(), rest.len() - 1, PhpVisibility::Private))
                    }
                    tail = rest;
                }
                return Err(self.create_error("Null terminator not found in private property"));
            },
            _ => Ok((1, length, PhpVisibility::Public))
        }
    }

    fn read_next(&mut self) -> Result<Option<(PhpTokenKind, usize)>, Error> {
        let mut current_position = self.position;
        loop {
            let mut buf = [0u8; 1];
            let read = self.reader.read(&mut buf)?;
            current_position += read;

            if read == 0 {
                return Ok(None);
            }

            let kind = match buf[0] {
                b'N' => PhpTokenKind::Null,
                b'b' => PhpTokenKind::Boolean,
                b'i' => PhpTokenKind::Integer,
                b'd' => PhpTokenKind::Float,
                b's' => PhpTokenKind::String,
                b'a' => PhpTokenKind::Array,
                b'O' => PhpTokenKind::Object,
                b'r' => PhpTokenKind::Reference,
                b'}' => PhpTokenKind::End,
                b'\n' => continue,
                _ => {
                    return Err(self.create_error(&format!(
                        "Unexpected PHP serialization token: '{}'",
                        buf[0] as char
                    )));
                }
            };

            return Ok(Some((kind, current_position)));
        }
    }

    pub fn consume_lookahead(&mut self) {
        if let Some((_, position)) = self.lookahead.take() {
            self.position = position;
        }
    }

    pub fn peek_token(&mut self) -> Result<Option<PhpTokenKind>, Error> {
        if let Some((token, _)) = self.lookahead {
            return Ok(Some(token));
        }

        match self.read_next() {
            Ok(Some((token, position))) => {
                self.lookahead = Some((token, position));
                Ok(Some(token))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn next_token<'buf>(
        &mut self,
        storage: &'buf mut Vec<u8>,
    ) -> Result<Option<PhpToken<'buf>>, Error> {
        let (kind, position) = match self.lookahead.take() {
            Some((kind, position)) => (kind, position),
            None => match self.read_next()? {
                Some((kind, position)) => (kind, position),
                None => return Ok(None),
            },
        };

        self.position = position;

        match kind {
            PhpTokenKind::End => Ok(Some(PhpToken::End)),
            PhpTokenKind::Null => {
                self.expect(b';')?;
                Ok(Some(PhpToken::Null))
            }
            PhpTokenKind::Boolean => {
                let mut buf = [0u8; 1];
                self.expect(b':')?;
                self.reader.read_exact(&mut buf)?;
                self.position += 1;

                let result = match buf[0] {
                    b'0' => PhpToken::Boolean(false),
                    b'1' => PhpToken::Boolean(true),
                    _ => {
                        return Err(self.create_error(&format!(
                            "Invalid boolean value: '{}'",
                            buf[0] as char
                        )));
                    }
                };

                self.expect(b';')?;
                Ok(Some(result))
            }
            PhpTokenKind::Integer => {
                self.expect(b':')?;
                storage.clear();

                let start_position = self.position;
                let bytes_read = self.reader.read_until(b';', storage)?;
                self.position += bytes_read;

                if !matches!(storage.last(), Some(b';')) {
                    return Err(self.create_error("Expected ';' after integer value"));
                }

                storage.pop();

                let s = std::str::from_utf8(storage)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                let x = s
                    .parse::<i32>()
                    .map_err(|e| Error::from(ErrorKind::ParseInt(e, start_position)))?;

                Ok(Some(PhpToken::Integer(x)))
            }
            PhpTokenKind::Float => {
                self.expect(b':')?;
                storage.clear();

                let start_position = self.position;
                let bytes_read = self.reader.read_until(b';', storage)?;
                self.position += bytes_read;

                if !matches!(storage.last(), Some(b';')) {
                    return Err(self.create_error("Expected ';' after float value"));
                }

                storage.pop();

                let s = std::str::from_utf8(storage)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                let x = s
                    .parse::<f64>()
                    .map_err(|e| Error::from(ErrorKind::ParseFloat(e, start_position)))?;

                Ok(Some(PhpToken::Float(x)))
            }
            PhpTokenKind::String => {
                self.expect(b':')?;
                let start_position = self.position;
                let (start, end, visibility ) = self.read_str(storage)?;
                self.expect(b';')?;

                let s = std::str::from_utf8(&storage[start..][..end])
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                Ok(Some(PhpToken::String(PhpStr::new(s, visibility))))
            }
            PhpTokenKind::Array => {
                self.expect(b':')?;
                storage.clear();

                let start_position = self.position;
                let bytes_read = self.reader.read_until(b'{', storage)?;

                if !matches!(storage.pop(), Some(b'{')) {
                    return Err(self.create_error("Expected '{' after array declaration"));
                }

                if !matches!(storage.pop(), Some(b':')) {
                    return Err(self.create_error("Expected ':' after array element count"));
                }

                self.position += bytes_read;
                let s = std::str::from_utf8(storage)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                let elements = s
                    .parse::<usize>()
                    .map_err(|e| Error::from(ErrorKind::ParseInt(e, start_position)))?;

                Ok(Some(PhpToken::Array { elements }))
            }
            PhpTokenKind::Object => {
                self.expect(b':')?;
                let start_position = self.position;
                let (class_name_start, class_name_len, visibility) = self.read_str(storage)?;
                self.expect(b':')?;

                let start_int = storage.len();
                let bytes_read = self.reader.read_until(b'{', storage)?;

                if !matches!(storage.pop(), Some(b'{')) {
                    return Err(self.create_error("Expected '{' after object property count"));
                }

                if !matches!(storage.pop(), Some(b':')) {
                    return Err(self.create_error("Expected ':' before object property count"));
                }

                self.position += bytes_read;
                let end_int = storage.len();
                let properties = &storage[start_int..end_int];

                let properties = std::str::from_utf8(properties)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                let properties = properties
                    .parse::<u32>()
                    .map_err(|e| Error::from(ErrorKind::ParseInt(e, start_position)))?;

                let class = &storage[class_name_start..][..class_name_len];
                let class = std::str::from_utf8(class)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;
                let class = PhpStr::new(class, visibility);

                Ok(Some(PhpToken::Object { class, properties }))
            }
            PhpTokenKind::Reference => {
                self.expect(b':')?;
                storage.clear();

                let start_position = self.position;
                let bytes_read = self.reader.read_until(b';', storage)?;
                self.position += bytes_read;

                if !matches!(storage.last(), Some(b';')) {
                    return Err(self.create_error("Expected ';' after integer value"));
                }

                storage.pop();

                let s = std::str::from_utf8(storage)
                    .map_err(|e| Error::from(ErrorKind::Utf8(e, start_position)))?;

                let x = s
                    .parse::<i32>()
                    .map_err(|e| Error::from(ErrorKind::ParseInt(e, start_position)))?;

                Ok(Some(PhpToken::Reference(x)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use rstest::rstest;

    fn validate_tokens(input: &[u8], expected_tokens: &[PhpToken<'_>]) {
        let cursor = Cursor::new(input);
        let mut parser = PhpParser::new(cursor);
        let mut token_index = 0;
        let mut storage = Vec::new();

        // Process all tokens from the parser
        while let Some(actual_token) = parser.next_token(&mut storage).unwrap() {
            // Check if we've run out of expected tokens
            assert!(
                token_index < expected_tokens.len(),
                "Unexpected extra token at position {}",
                parser.position
            );

            // Check if the token matches what we expect
            let expected_token = &expected_tokens[token_index];
            assert_eq!(
                &actual_token, expected_token,
                "Token mismatch at position {}: expected {:?}, got {:?} at position {}",
                token_index, expected_token, actual_token, parser.position
            );

            token_index += 1;
        }

        // Check if we've processed all expected tokens
        assert_eq!(
            token_index,
            expected_tokens.len(),
            "Missing expected tokens: {:?}",
            &expected_tokens[token_index..]
        );
    }

    #[rstest]
    #[case("i:42;", PhpToken::Integer(42))]
    #[case("i:-123;", PhpToken::Integer(-123))]
    #[case("i:0;", PhpToken::Integer(0))]
    #[case("i:2147483647;", PhpToken::Integer(i32::MAX))] 
    #[case("i:-2147483648;", PhpToken::Integer(i32::MIN))]
    fn test_parse_integer(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let input = input.as_bytes();
        let cursor = Cursor::new(input);
        let mut parser = PhpParser::new(cursor);
        let mut storage = Vec::new();
        assert_eq!(parser.next_token(&mut storage).unwrap(), Some(expected));
    }

    #[rstest]
    #[case("d:3.14;", PhpToken::Float(3.14))]
    #[case("d:-0.5;", PhpToken::Float(-0.5))]
    #[case("d:0.0;", PhpToken::Float(0.0))]
    #[case("d:10000000000;", PhpToken::Float(1.0E10))]
    #[case("d:1.0E+25;", PhpToken::Float(1.0E25))]
    #[case("d:-0.0025;", PhpToken::Float(-2.5E-3))]
    #[case("d:1.7976931348623157E+308;", PhpToken::Float(1.7976931348623157E+308))]  // f64::MAX
    #[case("d:2.2250738585072014E-308;", PhpToken::Float(2.2250738585072014E-308))]  // f64 smallest positive normal
    fn test_parse_float_rstest(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let input = input.as_bytes();
        let cursor = Cursor::new(input);
        let mut parser = PhpParser::new(cursor);
        let mut storage = Vec::new();
        assert_eq!(parser.next_token(&mut storage).unwrap(), Some(expected));
    }

    #[rstest]
    #[case("s:5:\"hello\";", PhpToken::String(PhpStr::public("hello")))]
    #[case("s:0:\"\";", PhpToken::String(PhpStr::public("")))]
    #[case("s:11:\"Hello World\";", PhpToken::String(PhpStr::public("Hello World")))]
    #[case("s:13:\"Special: \\\"\n\r\";", PhpToken::String(PhpStr::public("Special: \\\"\n\r")))]
    #[case("s:8:\"👋🌍\";", PhpToken::String(PhpStr::public("👋🌍")))]
    #[case("s:10:\"0123456789\";", PhpToken::String(PhpStr::public("0123456789")))]
    #[case("s:19:\"\0MyClass\0privateVar\";", PhpToken::String(PhpStr::private("privateVar")))]
    #[case("s:11:\"\0MyClass\0pv\";", PhpToken::String(PhpStr::private("pv")))]
    #[case("s:15:\"\0*\0protectedVar\";", PhpToken::String(PhpStr::protected("protectedVar")))]
    #[case("s:7:\"\0*\0pwho\";", PhpToken::String(PhpStr::protected("pwho")))]
    fn test_parse_string(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let input = input.as_bytes();
        let cursor = Cursor::new(input);
        let mut parser = PhpParser::new(cursor);
        let mut storage = Vec::new();
        assert_eq!(parser.next_token(&mut storage).unwrap(), Some(expected));
    }

    #[test]
    fn test_parse_null() {
        let input = b"N;";
        let expected = [PhpToken::Null];
        validate_tokens(input, &expected);
    }

    #[test]
    fn test_parse_boolean() {
        let input = b"b:0;b:1;";
        let expected = [PhpToken::Boolean(false), PhpToken::Boolean(true)];
        validate_tokens(input, &expected);
    }

    // #[test]
    // fn test_parse_array() {
    //     let input = b"a:3:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";i:2;s:3:\"baz\";}";
    //     let expected = [
    //         PhpToken::Array { elements: 3 },
    //         PhpToken::Integer(0),
    //         PhpToken::String("foo"),
    //         PhpToken::Integer(1),
    //         PhpToken::String("bar"),
    //         PhpToken::Integer(2),
    //         PhpToken::String("baz"),
    //         PhpToken::End,
    //     ];
    //     validate_tokens(input, &expected);
    // }

    // #[test]
    // fn test_parse_object() {
    //     let input = b"O:3:\"Foo\":2:{s:3:\"bar\";i:42;s:3:\"baz\";s:5:\"hello\";}";
    //     let expected = [
    //         PhpToken::Object {
    //             class: "Foo",
    //             properties: 2,
    //         },
    //         PhpToken::String("bar"),
    //         PhpToken::Integer(42),
    //         PhpToken::String("baz"),
    //         PhpToken::String("hello"),
    //         PhpToken::End,
    //     ];
    //     validate_tokens(input, &expected);
    // }

    // #[test]
    // fn test_parse_complex_structure() {
    //     let input = b"a:2:{i:0;a:2:{s:3:\"foo\";i:42;s:3:\"bar\";b:1;}i:1;O:3:\"Xyz\":1:{s:4:\"prop\";s:5:\"value\";}}";
    //     let expected = [
    //         PhpToken::Array { elements: 2 },
    //         PhpToken::Integer(0),
    //         PhpToken::Array { elements: 2 },
    //         PhpToken::String("foo"),
    //         PhpToken::Integer(42),
    //         PhpToken::String("bar"),
    //         PhpToken::Boolean(true),
    //         PhpToken::End,
    //         PhpToken::Integer(1),
    //         PhpToken::Object {
    //             class: "Xyz",
    //             properties: 1,
    //         },
    //         PhpToken::String("prop"),
    //         PhpToken::String("value"),
    //         PhpToken::End,
    //         PhpToken::End,
    //     ];
    //     validate_tokens(input, &expected);
    // }

    fn error_case(input: &[u8]) -> Error {
        let cursor = Cursor::new(input);
        let mut parser = PhpParser::new(cursor);
        let mut storage = Vec::new();

        loop {
            match parser.next_token(&mut storage) {
                Ok(Some(_)) => {}
                Ok(None) => panic!("expected an error"),
                Err(e) => {
                    return e;
                }
            }
        }
    }

    #[test]
    fn test_invalid_token() {
        let input = b"x:invalid;";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Unexpected PHP serialization token"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_invalid_boolean() {
        let input = b"b:2;";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Invalid boolean value"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_malformed_integer() {
        let input = b"i:abc;";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Integer parse error"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_malformed_float() {
        let input = b"d:3.14.15;";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Float parse error"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_invalid_string_length() {
        let input = b"s:abc:\"hello\";";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Integer parse error"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_string_length_mismatch() {
        let input = b"s:10:\"hello\";";
        let err = error_case(input).to_string();
        assert!(
            err.contains("String is not properly enclosed") || err.contains("length mismatch"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_missing_string_terminator() {
        let input = b"s:5:\"hello";
        let err = error_case(input).to_string();
        assert!(
            err.contains("unexpected EOF") || err.contains("IO error"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_missing_semicolon() {
        let input = b"i:42";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Expected ';' after integer value"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_array_missing_open_brace() {
        let input = b"a:3:i:0;s:3:\"foo\";}";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Expected '{' after array declaration"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_object_missing_properties() {
        let input = b"O:3:\"Foo\":;";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Expected ':' before object property count"),
            "Unexpected error message: {}",
            err
        );
    }

    #[test]
    fn test_object_invalid_property_count() {
        let input = b"O:3:\"Foo\":xyz{";
        let err = error_case(input).to_string();
        assert!(
            err.contains("Integer parse error"),
            "Unexpected error message: {}",
            err
        );
    }
}
