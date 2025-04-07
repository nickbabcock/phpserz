use crate::errors::{Error, ErrorKind};

/// A byte string that is conventionally UTF-8.
///
/// UTF-8 decoding is an expensive operation and PHP strings aren't guaranteed
/// to be valid UTF-8, so we store them as a bytes, and defer decoding onto the
/// caller.
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct PhpBstr<'a> {
    data: &'a [u8],
}

impl<'a> PhpBstr<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub const fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    pub fn to_str(&self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.data).map_err(|e| Error::from(ErrorKind::Utf8(e)))
    }

    pub fn to_property(&self) -> Result<(&'a str, PhpVisibility), Error> {
        let (data, visibility) = match self.data {
            [0, b'*', 0, contents @ ..] => (contents, PhpVisibility::Protected),
            [0, tail @ ..] => {
                let mut tail = tail;
                loop {
                    match tail.split_first() {
                        Some((0, contents)) => break (contents, PhpVisibility::Private),
                        Some((_, contents)) => tail = contents,
                        None => break (self.data, PhpVisibility::Public),
                    }
                }
            }
            _ => (self.data, PhpVisibility::Public),
        };

        let result = std::str::from_utf8(data).map_err(|e| Error::from(ErrorKind::Utf8(e)))?;
        Ok((result, visibility))
    }
}

/// The visibility of a property in a PHP object.
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
    String(PhpBstr<'a>),
    Array { elements: u32 },
    Object { class: PhpBstr<'a>, properties: u32 },
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

pub struct PhpParser<'a> {
    data: &'a [u8],
    lookahead: Option<(PhpTokenKind, usize)>,
    position: usize,
}

impl<'a> PhpParser<'a> {
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            lookahead: None,
            position: 0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        let (&c, rest) = self
            .data
            .split_first()
            .ok_or_else(|| Error::from(ErrorKind::Eof))?;

        if c != expected {
            return Err(Error::from(ErrorKind::MismatchByte {
                expected,
                found: c,
                position: self.position,
            }));
        }

        self.position += 1;
        self.data = rest;
        Ok(())
    }

    #[inline]
    fn read_next(&mut self) -> Result<Option<(PhpTokenKind, usize)>, Error> {
        let mut current_position = self.position;
        loop {
            let Some((&c, rest)) = self.data.split_first() else {
                return Ok(None);
            };

            self.data = rest;
            current_position += 1;
            let kind = match c {
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
                    return Err(Error::from(ErrorKind::UnexpectedByte {
                        found: c,
                        position: self.position,
                    }));
                }
            };

            return Ok(Some((kind, current_position)));
        }
    }

    pub const fn consume_lookahead(&mut self) {
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

    #[inline]
    pub fn read_token(&mut self) -> Result<PhpToken<'a>, Error> {
        let token = self.next_token()?;
        Ok(token.ok_or(ErrorKind::Eof)?)
    }

    #[inline]
    pub fn next_token(&mut self) -> Result<Option<PhpToken<'a>>, Error> {
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
                self.expect(b':')?;

                let (&c, rest) = self
                    .data
                    .split_first()
                    .ok_or_else(|| Error::from(ErrorKind::Eof))?;

                let token = match c {
                    b'0' => PhpToken::Boolean(false),
                    b'1' => PhpToken::Boolean(true),
                    _ => {
                        return Err(Error::from(ErrorKind::UnexpectedByte {
                            found: c,
                            position: self.position,
                        }));
                    }
                };

                self.data = rest;
                self.position += 1;
                self.expect(b';')?;
                Ok(Some(token))
            }
            PhpTokenKind::Integer => {
                self.expect(b':')?;
                let (int, rest) = to_i32(self.data).map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.data = rest;
                self.position += bytes_read;
                Ok(Some(PhpToken::Integer(int)))
            }
            PhpTokenKind::Float => {
                self.expect(b':')?;

                let (num, len) = fast_float2::parse_partial(self.data).map_err(|_| {
                    Error::from(ErrorKind::InvalidNumber {
                        position: self.position,
                    })
                })?;

                self.position += len;
                self.data = &self.data[len..];
                self.expect(b';')?;
                Ok(Some(PhpToken::Float(num)))
            }
            PhpTokenKind::String => {
                self.expect(b':')?;
                let (s, rest) = read_str(self.data).map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.position += bytes_read;
                self.data = rest;
                self.expect(b';')?;
                Ok(Some(PhpToken::String(s)))
            }
            PhpTokenKind::Array => {
                self.expect(b':')?;
                let (elements, rest) = read_u32(self.data, b':').map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.position += bytes_read;
                self.data = rest;
                self.expect(b'{')?;
                Ok(Some(PhpToken::Array { elements }))
            }
            PhpTokenKind::Object => {
                self.expect(b':')?;
                let (class, rest) = read_str(self.data).map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.position += bytes_read;
                self.data = rest;
                self.expect(b':')?;

                let (properties, rest) =
                    read_u32(self.data, b':').map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.position += bytes_read;
                self.data = rest;
                self.expect(b'{')?;

                Ok(Some(PhpToken::Object { class, properties }))
            }
            PhpTokenKind::Reference => {
                self.expect(b':')?;
                let (int, rest) = to_i32(self.data).map_err(|e| self.map_error(e))?;
                let bytes_read = self.data.len() - rest.len();
                self.position += bytes_read;
                Ok(Some(PhpToken::Reference(int)))
            }
        }
    }

    #[cold]
    fn map_error(&self, error: ScalarError) -> Error {
        match error {
            ScalarError::StringTooLong => (ErrorKind::StringTooLong {
                position: self.position,
            })
            .into(),
            ScalarError::MissingQuotes => (ErrorKind::MissingQuotes {
                position: self.position,
            })
            .into(),
            ScalarError::Empty => (ErrorKind::Empty {
                position: self.position,
            })
            .into(),
            ScalarError::Overflow => (ErrorKind::Overflow {
                position: self.position,
            })
            .into(),
            ScalarError::Invalid => (ErrorKind::InvalidNumber {
                position: self.position,
            })
            .into(),
            ScalarError::Eof => ErrorKind::Eof.into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
enum ScalarError {
    StringTooLong,
    MissingQuotes,
    Empty,
    Overflow,
    Invalid,
    Eof,
}

#[inline]
fn read_str(data: &[u8]) -> Result<(PhpBstr, &[u8]), ScalarError> {
    let (len, data) = read_u32(data, b':')?;
    let len = len as usize;
    let Some((contents, rest)) = data.split_at_checked(len + 2) else {
        return Err(ScalarError::StringTooLong);
    };

    match contents {
        [b'"', contents @ .., b'"'] => Ok((PhpBstr::new(contents), rest)),
        _ => Err(ScalarError::MissingQuotes),
    }
}

#[inline]
fn read_u32(mut data: &[u8], delimiter: u8) -> Result<(u32, &[u8]), ScalarError> {
    let mut result = 0u64;
    let original_len = data.len();
    while let Some((&c, rest)) = data.split_first() {
        if c == delimiter {
            let bytes_read = original_len - rest.len();

            if bytes_read == 0 {
                return Err(ScalarError::Empty);
            }

            // Check for overflow
            if result > u64::from(u32::MAX) || bytes_read > 11 {
                return Err(ScalarError::Overflow);
            }

            return Ok((result as u32, rest));
        }

        if !c.is_ascii_digit() {
            return Err(ScalarError::Invalid);
        }

        result = result.wrapping_mul(10);
        result = result.wrapping_add(u64::from(c - b'0'));
        data = rest;
    }

    Err(ScalarError::Eof)
}

#[inline]
fn to_i32(d: &[u8]) -> Result<(i32, &[u8]), ScalarError> {
    let mut integer_part = d;

    let Some((&c, mut data)) = d.split_first() else {
        return Err(ScalarError::Empty);
    };

    let mut sign = 1;

    let mut result = if c.is_ascii_digit() {
        u64::from(c - b'0')
    } else if c == b'-' {
        integer_part = data;
        sign = -1;
        0
    } else {
        return Err(ScalarError::Invalid);
    };

    let original_len = integer_part.len();
    while let Some((&c, rest)) = data.split_first() {
        if c == b';' {
            let bytes_read = original_len - rest.len();
            if bytes_read == 0 {
                return Err(ScalarError::Empty);
            }

            // Check for overflow
            if result > (i32::MAX as u64) + 1 || bytes_read > 11 {
                return Err(ScalarError::Overflow);
            }

            let Ok(val) = i64::try_from(result)
                .map(|x| sign * x)
                .and_then(i32::try_from)
            else {
                return Err(ScalarError::Overflow);
            };
            return Ok((val, rest));
        } else if !c.is_ascii_digit() {
            return Err(ScalarError::Invalid);
        }

        result = result.wrapping_mul(10);
        result = result.wrapping_add(u64::from(c - b'0'));
        data = rest;
    }

    Err(ScalarError::Eof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn validate_tokens(input: &[u8], expected_tokens: &[PhpToken<'_>]) {
        let mut parser = PhpParser::new(input);
        let mut token_index = 0;

        // Process all tokens from the parser
        while let Some(actual_token) = parser.next_token().unwrap() {
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
        let mut parser = PhpParser::new(input.as_bytes());
        assert_eq!(parser.next_token().unwrap(), Some(expected));
    }

    #[rstest]
    #[case("d:3.14;", PhpToken::Float(3.14))]
    #[case("d:-0.5;", PhpToken::Float(-0.5))]
    #[case("d:0.0;", PhpToken::Float(0.0))]
    #[case("d:10000000000;", PhpToken::Float(1.0E10))]
    #[case("d:1.0E+25;", PhpToken::Float(1.0E25))]
    #[case("d:-0.0025;", PhpToken::Float(-2.5E-3))]
    #[case(
        "d:1.7976931348623157E+308;",
        PhpToken::Float(1.797_693_134_862_315_7E308)
    )] // f64::MAX
    #[case("d:2.2250738585072014E-308;", PhpToken::Float(2.2250738585072014E-308))] // f64 smallest positive normal
    fn test_parse_float(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let mut parser = PhpParser::new(input.as_bytes());
        assert_eq!(parser.next_token().unwrap(), Some(expected));
    }

    #[rstest]
    #[case("s:5:\"hello\";", PhpToken::String(PhpBstr::new(b"hello")))]
    #[case("s:0:\"\";", PhpToken::String(PhpBstr::new(b"")))]
    #[case(
        "s:11:\"Hello World\";",
        PhpToken::String(PhpBstr::new(b"Hello World"))
    )]
    #[case(
        "s:13:\"Special: \\\"\n\r\";",
        PhpToken::String(PhpBstr::new(b"Special: \\\"\n\r"))
    )]
    #[case("s:8:\"👋🌍\";", PhpToken::String(PhpBstr::new("👋🌍".as_bytes())))]
    #[case("s:10:\"0123456789\";", PhpToken::String(PhpBstr::new(b"0123456789")))]
    #[case(
        "s:19:\"\0MyClass\0privateVar\";",
        PhpToken::String(PhpBstr::new(b"\0MyClass\0privateVar"))
    )]
    #[case(
        "s:11:\"\0MyClass\0pv\";",
        PhpToken::String(PhpBstr::new(b"\0MyClass\0pv"))
    )]
    #[case(
        "s:15:\"\0*\0protectedVar\";",
        PhpToken::String(PhpBstr::new(b"\0*\0protectedVar"))
    )]
    #[case("s:7:\"\0*\0pwho\";", PhpToken::String(PhpBstr::new(b"\0*\0pwho")))]
    fn test_parse_string(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let mut parser = PhpParser::new(input.as_bytes());
        assert_eq!(parser.next_token().unwrap(), Some(expected));
    }

    #[rstest]
    #[case("s:5:\"hello\";", ("hello", PhpVisibility::Public))]
    #[case(
        "s:19:\"\0MyClass\0privateVar\";",
        ("privateVar", PhpVisibility::Private)
    )]
    #[case("s:17:\"\0MySecretClass\0pv\";", ("pv", PhpVisibility::Private))]
    #[case(
        "s:15:\"\0*\0protectedVar\";",
        ("protectedVar", PhpVisibility::Protected)
    )]
    #[case("s:7:\"\0*\0pwho\";", ("pwho", PhpVisibility::Protected))]
    fn test_parse_property(#[case] input: &str, #[case] expected: (&str, PhpVisibility)) {
        let mut parser = PhpParser::new(input.as_bytes());
        let token = parser.next_token().unwrap().unwrap();
        let PhpToken::String(bstr) = token else {
            panic!("Expected a string token");
        };

        assert_eq!(bstr.to_property().unwrap(), expected);
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

    #[test]
    fn test_parse_array() {
        let input = b"a:3:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";i:2;s:3:\"baz\";}";
        let expected = [
            PhpToken::Array { elements: 3 },
            PhpToken::Integer(0),
            PhpToken::String(PhpBstr::new(b"foo")),
            PhpToken::Integer(1),
            PhpToken::String(PhpBstr::new(b"bar")),
            PhpToken::Integer(2),
            PhpToken::String(PhpBstr::new(b"baz")),
            PhpToken::End,
        ];
        validate_tokens(input, &expected);
    }

    #[test]
    fn test_parse_object() {
        let input = b"O:3:\"Foo\":2:{s:3:\"bar\";d:20.3;s:3:\"baz\";s:5:\"hello\";}";
        let expected = [
            PhpToken::Object {
                class: PhpBstr::new(b"Foo"),
                properties: 2,
            },
            PhpToken::String(PhpBstr::new(b"bar")),
            PhpToken::Float(20.3),
            PhpToken::String(PhpBstr::new(b"baz")),
            PhpToken::String(PhpBstr::new(b"hello")),
            PhpToken::End,
        ];
        validate_tokens(input, &expected);
    }

    #[test]
    fn test_parse_complex_structure() {
        let input = b"a:2:{i:0;a:2:{s:3:\"foo\";i:42;s:3:\"bar\";b:1;}i:1;O:3:\"Xyz\":1:{s:4:\"prop\";s:5:\"value\";}}";
        let expected = [
            PhpToken::Array { elements: 2 },
            PhpToken::Integer(0),
            PhpToken::Array { elements: 2 },
            PhpToken::String(PhpBstr::new(b"foo")),
            PhpToken::Integer(42),
            PhpToken::String(PhpBstr::new(b"bar")),
            PhpToken::Boolean(true),
            PhpToken::End,
            PhpToken::Integer(1),
            PhpToken::Object {
                class: PhpBstr::new(b"Xyz"),
                properties: 1,
            },
            PhpToken::String(PhpBstr::new(b"prop")),
            PhpToken::String(PhpBstr::new(b"value")),
            PhpToken::End,
            PhpToken::End,
        ];
        validate_tokens(input, &expected);
    }

    fn error_case(input: &[u8]) -> Result<(), Error> {
        let mut parser = PhpParser::new(input);
        loop {
            match parser.next_token() {
                Ok(Some(_)) => {}
                Ok(None) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    #[test]
    fn test_invalid_token() {
        let input = b"x:invalid;";
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid token"
        );
    }

    #[rstest]
    #[case(b"b:2;")]
    #[case(b"b:3;")]
    #[case(b"b:-1;")]
    #[case(b"b:10;")]
    #[case(b"b:text;")]
    fn test_invalid_boolean_values(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid boolean value: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"i:abc;")]
    #[case(b"i:--1;")]
    #[case(b"i:1a;")]
    #[case(b"i: 42;")]
    #[case(b"i:+42;")]
    #[case(b"i:2147483648;")] // i32::MAX + 1
    #[case(b"i:9999999999;")] // Definitely too big for i32
    #[case(b"i:-2147483649;")] // i32::MIN - 1
    #[case(b"i:-9999999999;")] // Definitely too small for i32
    fn test_invalid_integer_values(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid integer: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"d:3.14.15;")]
    #[case(b"d:invalid;")]
    #[case(b"d:3,14;")]
    #[case(b"d:--1.0;")]
    fn test_invalid_float_values(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid float: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"s:abc:\"hello\";")]
    #[case(b"s:-1:\"hello\";")]
    #[case(b"s:9999999999:\"hello\";")]
    fn test_invalid_string_length_format(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid string length format: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"s:10:\"hello\";")] // String length exceeds available data
    #[case(b"s:3:\"hello\";")] // String length is less than actual content
    #[case(b"s:5:\"hello;")] // Missing closing quote
    #[case(b"s:5:hello\";")] // Missing opening quote
    #[case(b"s:1000:\"hello\";")] // Extreme case: length much larger than input
    fn test_string_content_mismatch(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for string content mismatch: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"s:5:\"hello")]
    #[case(b"i:42")]
    #[case(b"b:1")]
    fn test_unexpected_end_of_input(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for unexpected end of input: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"a:3:i:0;s:3:\"foo\";}")]
    #[case(b"a:-1:{i:0;s:3:\"foo\";}")]
    #[case(b"a:9999999999:{i:0;s:3:\"foo\";}")]
    fn test_invalid_array_structure(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid array structure: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"O:3:\"Foo\":;")]
    #[case(b"O:3:\"Foo\":xyz{")]
    #[case(b"O:-1:\"Foo\":2:{")]
    #[case(b"O:3:\"Foo\":-1:{")]
    #[case(b"O:9999999999:\"Foo\":2:{")]
    #[case(b"O:3:\"Foo\":9999999999:{")]
    fn test_invalid_object_structure(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid object structure: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[rstest]
    #[case(b"r:;")]
    #[case(b"r:xyz;")]
    #[case(b"r:9999999999;")]
    #[case(b"r:-9999999999;")]
    fn test_invalid_reference(#[case] input: &[u8]) {
        assert!(
            error_case(input).is_err(),
            "Expected an error for invalid reference: {}",
            String::from_utf8_lossy(input)
        );
    }

    #[test]
    fn test_position_tracking() {
        let input = b"i:42;s:5:\"hello\";";
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.position(), 0, "Initial position should be 0");

        // Parse the integer
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(42));
        assert_eq!(
            parser.position(),
            5,
            "Position after parsing 'i:42;' should be 5"
        );

        // Parse the string
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::String(PhpBstr::new(b"hello")));
        assert_eq!(
            parser.position(),
            17,
            "Position after parsing full input should be 17"
        );

        // Verify no more tokens
        assert!(parser.next_token().unwrap().is_none());
        assert_eq!(
            parser.position(),
            17,
            "Position should not change when reaching end of input"
        );
    }

    #[test]
    fn test_position_with_peek() {
        let input = b"i:42;s:5:\"hello\";";
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.position(), 0, "Initial position should be 0");

        // Peek the first token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::Integer);
        assert_eq!(
            parser.position(),
            0,
            "Position should not change after peeking"
        );

        // Read the token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(42));
        assert_eq!(
            parser.position(),
            5,
            "Position should update after reading token"
        );

        // Peek the second token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::String);
        assert_eq!(
            parser.position(),
            5,
            "Position should not change after peeking"
        );

        // Read the token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::String(PhpBstr::new(b"hello")));
        assert_eq!(
            parser.position(),
            17,
            "Position after parsing full input should be 17"
        );
    }

    #[test]
    fn test_position_with_consume_lookahead() {
        let input = b"N;a:0:{}";
        let mut parser = PhpParser::new(input);

        // Peek the null token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::Null);
        assert_eq!(
            parser.position(),
            0,
            "Position should not change after peeking"
        );

        // Consume the lookahead
        parser.consume_lookahead();
        assert_eq!(
            parser.position(),
            1,
            "Position should update to token position after consume_lookahead"
        );

        // Read the null token
        assert_eq!(parser.data[0], b';');
        parser.expect(b';').unwrap();

        // Peek at array token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::Array);

        // Read array token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Array { elements: 0 });
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::End);
        parser.consume_lookahead();

        assert_eq!(parser.next_token().unwrap(), None);
    }

    #[test]
    fn test_multiple_peeks() {
        let input = b"i:42;s:5:\"hello\";";
        let mut parser = PhpParser::new(input);

        // Peek the first token multiple times
        let token_kind1 = parser.peek_token().unwrap().unwrap();
        let token_kind2 = parser.peek_token().unwrap().unwrap();
        let token_kind3 = parser.peek_token().unwrap().unwrap();

        assert_eq!(token_kind1, PhpTokenKind::Integer);
        assert_eq!(token_kind2, PhpTokenKind::Integer);
        assert_eq!(token_kind3, PhpTokenKind::Integer);
        assert_eq!(
            parser.position(),
            0,
            "Position should not change after multiple peeks"
        );

        // Read the token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(42));
        assert_eq!(
            parser.position(),
            5,
            "Position should update after reading token"
        );
    }

    #[test]
    fn test_peek_after_next() {
        let input = b"i:42;s:5:\"hello\";b:1;";
        let mut parser = PhpParser::new(input);

        // Read the first token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(42));
        assert_eq!(parser.position(), 5);

        // Peek the next token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::String);
        assert_eq!(
            parser.position(),
            5,
            "Position should not change after peeking"
        );

        // Read the second token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::String(PhpBstr::new(b"hello")));
        assert_eq!(parser.position(), 17);

        // Peek the third token
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::Boolean);
        assert_eq!(
            parser.position(),
            17,
            "Position should not change after peeking"
        );

        // Read the third token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Boolean(true));
        assert_eq!(parser.position(), 21);
    }

    #[test]
    fn test_position_with_complex_structure() {
        let input = b"a:1:{i:0;s:5:\"hello\";}";
        let mut parser = PhpParser::new(input);

        // Read array start
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Array { elements: 1 });
        assert_eq!(
            parser.position(),
            5,
            "Position after array start should be 5"
        );

        // Peek next token (should be integer)
        let token_kind = parser.peek_token().unwrap().unwrap();
        assert_eq!(token_kind, PhpTokenKind::Integer);
        assert_eq!(
            parser.position(),
            5,
            "Position should not change after peeking"
        );

        // Read integer
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(0));
        assert_eq!(parser.position(), 9, "Position after integer should be 9");

        // Read string
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::String(PhpBstr::new(b"hello")));
        assert_eq!(parser.position(), 21, "Position after string should be 21");

        // Read end token
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::End);
        assert_eq!(
            parser.position(),
            22,
            "Position after end token should be 22"
        );
    }

    #[test]
    fn test_position_with_nested_structures() {
        let input = b"a:1:{i:0;a:1:{i:0;s:5:\"hello\";}}";
        let mut parser = PhpParser::new(input);

        // Read outer array start
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Array { elements: 1 });
        let pos_after_outer_array = parser.position();

        // Read key
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Integer(0));

        // Read inner array start
        let token = parser.next_token().unwrap().unwrap();
        assert_eq!(token, PhpToken::Array { elements: 1 });
        let pos_after_inner_array = parser.position();
        assert!(
            pos_after_inner_array > pos_after_outer_array,
            "Position should increase after reading inner array"
        );

        let tokens = std::iter::from_fn(|| parser.next_token().unwrap());

        assert_eq!(tokens.count(), 4);
        assert_eq!(
            parser.position(),
            input.len(),
            "Position should be at the end of input"
        );
    }
}
