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
    /// Create a new byte string from a slice of bytes.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get the underlying bytes of the byte string.
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    /// Convert the byte string to a string.
    pub fn to_str(self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.data).map_err(|e| Error::from(ErrorKind::Utf8(e)))
    }

    /// Convert the byte string to a property name with a visibility.
    ///
    /// This assumes the caller is deserializing a PHP property which can either be public, protected, or private.
    ///
    /// > Private properties are prefixed with `\0ClassName\0` and protected properties with `\0*\0`.
    ///
    /// Source: <https://www.phpinternalsbook.com/php5/classes_objects/serialization.html>
    pub fn to_property(self) -> PhpProperty<'a> {
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

        PhpProperty {
            name: data,
            visibility,
        }
    }
}

/// The visibility of a property in a PHP object.
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum PhpVisibility {
    Public,
    Protected,
    Private,
}

/// A PHP object property with its name and visibility.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PhpProperty<'a> {
    name: &'a [u8],
    visibility: PhpVisibility,
}

impl<'a> PhpProperty<'a> {
    /// Get the underlying bytes of the property name.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.name
    }

    /// Convert the property name to a string.
    #[inline]
    pub fn to_str(self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.name).map_err(|e| Error::from(ErrorKind::Utf8(e)))
    }

    /// Get the visibility of the property.
    #[inline]
    pub const fn visibility(&self) -> PhpVisibility {
        self.visibility
    }
}

/// A token in the PHP serialized format.
#[derive(Debug, PartialEq)]
pub enum PhpToken<'a> {
    /// The null token.
    Null,

    /// The boolean token.
    Boolean(bool),

    /// The integer token.
    Integer(i64),

    /// The float token.
    Float(f64),

    /// The string token.
    String(PhpBstr<'a>),

    /// The array token.
    Array { elements: u32 },

    /// The object token.
    Object { class: PhpBstr<'a>, properties: u32 },

    /// A custom-serialized object token.
    CustomObject {
        class: PhpBstr<'a>,
        payload: PhpBstr<'a>,
    },

    /// The end of an array or object.
    End,

    /// The reference token.
    Reference(i64),
}

/// The kind of token without data.
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum PhpTokenKind {
    Null,
    Boolean,
    Integer,
    Float,
    String,
    Array,
    Object,
    CustomObject,
    End,
    Reference,
}

/// A parser for the PHP serialized format.
#[derive(Debug)]
pub struct PhpParser<'a> {
    data: &'a [u8],
    original_len: usize,
}

impl<'a> PhpParser<'a> {
    /// Create a new parser from a slice of bytes.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            original_len: data.len(),
            data,
        }
    }

    /// Get the current position of the parser.
    #[must_use]
    pub fn position(&self) -> usize {
        self.original_len - self.data.len()
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
                position: self.position(),
            }));
        }

        self.data = rest;
        Ok(())
    }

    #[inline]
    fn read_next(&mut self) -> Result<Option<PhpTokenKind>, Error> {
        let position = self.position();
        let Some((&c, rest)) = self.data.split_first() else {
            return Ok(None);
        };

        self.data = rest;
        let kind = match c {
            b'N' => PhpTokenKind::Null,
            b'b' => PhpTokenKind::Boolean,
            b'i' => PhpTokenKind::Integer,
            b'd' => PhpTokenKind::Float,
            b's' => PhpTokenKind::String,
            b'a' => PhpTokenKind::Array,
            b'O' => PhpTokenKind::Object,
            b'C' => PhpTokenKind::CustomObject,
            b'r' => PhpTokenKind::Reference,
            b'}' => PhpTokenKind::End,
            _ => {
                return Err(Error::from(ErrorKind::UnexpectedByte {
                    found: c,
                    position,
                }));
            }
        };

        self.data = rest;
        Ok(Some(kind))
    }

    /// Peek at the kind of the next token without consuming it.
    ///
    /// Useful for detecting end of arrays and objects.
    ///
    /// Calling this function multiple times will return the same token.
    ///
    /// ```rust
    /// use phpserz::{PhpParser, PhpToken, PhpTokenKind};
    /// let mut parser = PhpParser::new(b"i:42;");
    /// assert_eq!(parser.peek_token().unwrap(), Some(PhpTokenKind::Integer));
    /// assert_eq!(parser.next_token().unwrap(), Some(PhpToken::Integer(42)));
    /// ```
    pub fn peek_token(&mut self) -> Result<Option<PhpTokenKind>, Error> {
        let Some((&c, _rest)) = self.data.split_first() else {
            return Ok(None);
        };

        let kind = match c {
            b'N' => PhpTokenKind::Null,
            b'b' => PhpTokenKind::Boolean,
            b'i' => PhpTokenKind::Integer,
            b'd' => PhpTokenKind::Float,
            b's' => PhpTokenKind::String,
            b'a' => PhpTokenKind::Array,
            b'O' => PhpTokenKind::Object,
            b'C' => PhpTokenKind::CustomObject,
            b'r' => PhpTokenKind::Reference,
            b'}' => PhpTokenKind::End,
            _ => {
                return Err(Error::from(ErrorKind::UnexpectedByte {
                    found: c,
                    position: self.position(),
                }));
            }
        };

        Ok(Some(kind))
    }

    /// Reads the next token, and will error if the end of the input is reached.
    #[inline]
    pub fn read_token(&mut self) -> Result<PhpToken<'a>, Error> {
        let kind = self.read_next()?.ok_or(ErrorKind::Eof)?;
        self.parse_token_body(kind)
    }

    /// Attempt to read the next token. Will return Ok(None) if the end of the input is reached.
    ///
    /// ```rust
    /// use phpserz::{PhpParser, PhpToken, PhpTokenKind};
    /// let mut parser = PhpParser::new(b"i:42;");
    /// assert_eq!(parser.next_token().unwrap(), Some(PhpToken::Integer(42)));
    /// ```
    #[inline]
    pub fn next_token(&mut self) -> Result<Option<PhpToken<'a>>, Error> {
        let kind = match self.read_next()? {
            Some(kind) => kind,
            None => return Ok(None),
        };

        self.parse_token_body(kind).map(Some)
    }

    #[inline]
    fn parse_token_body(&mut self, kind: PhpTokenKind) -> Result<PhpToken<'a>, Error> {
        match kind {
            PhpTokenKind::End => Ok(PhpToken::End),
            PhpTokenKind::Null => {
                self.expect(b';')?;
                Ok(PhpToken::Null)
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
                            position: self.position(),
                        }));
                    }
                };

                self.data = rest;
                self.expect(b';')?;
                Ok(token)
            }
            PhpTokenKind::Integer => {
                self.expect(b':')?;
                let (int, rest) = to_i64(self.data).map_err(|e| self.map_error(e))?;
                self.data = rest;
                Ok(PhpToken::Integer(int))
            }
            PhpTokenKind::Float => {
                self.expect(b':')?;

                let (num, len) = fast_float2::parse_partial(self.data).map_err(|_| {
                    Error::from(ErrorKind::InvalidNumber {
                        position: self.position(),
                    })
                })?;

                self.data = &self.data[len..];
                self.expect(b';')?;
                Ok(PhpToken::Float(num))
            }
            PhpTokenKind::String => {
                self.expect(b':')?;
                let (s, rest) = read_str(self.data).map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b';')?;
                Ok(PhpToken::String(s))
            }
            PhpTokenKind::Array => {
                self.expect(b':')?;
                let (elements, rest) = read_u32(self.data, b':').map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b'{')?;
                Ok(PhpToken::Array { elements })
            }
            PhpTokenKind::Object => {
                self.expect(b':')?;
                let (class, rest) = read_str(self.data).map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b':')?;

                let (properties, rest) =
                    read_u32(self.data, b':').map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b'{')?;

                Ok(PhpToken::Object { class, properties })
            }
            PhpTokenKind::CustomObject => {
                self.expect(b':')?;
                let (class, rest) = read_str(self.data).map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b':')?;

                let (payload_len, rest) =
                    read_u32(self.data, b':').map_err(|e| self.map_error(e))?;
                self.data = rest;
                self.expect(b'{')?;

                let Some((payload, rest)) = self.data.split_at_checked(payload_len as usize) else {
                    return Err(ErrorKind::Eof.into());
                };
                self.data = rest;
                self.expect(b'}')?;

                Ok(PhpToken::CustomObject {
                    class,
                    payload: PhpBstr::new(payload),
                })
            }
            PhpTokenKind::Reference => {
                self.expect(b':')?;
                let (int, rest) = to_i64(self.data).map_err(|e| self.map_error(e))?;
                self.data = rest;
                Ok(PhpToken::Reference(int))
            }
        }
    }

    /// Try to read the next token as a string up to 99 characters long
    #[inline]
    pub(crate) fn try_read_str(&mut self) -> Option<PhpBstr<'a>> {
        let data = self.data;
        let d = data.get(..16)?;
        if d[0] != b's' || d[1] != b':' || !d[2].is_ascii_digit() {
            return None;
        }

        if d[3] == b':' {
            let len = usize::from(d[2] - b'0');
            let end = 5 + len;
            if d[4] != b'"' || d[end] != b'"' || d[end + 1] != b';' {
                return None;
            }

            self.data = &data[end + 2..];
            return Some(PhpBstr::new(&d[5..end]));
        }

        if d[4] != b':' || !d[3].is_ascii_digit() || d[5] != b'"' {
            return None;
        }

        let len = usize::from(d[2] - b'0') * 10 + usize::from(d[3] - b'0');
        let end = 6 + len;
        let (s, rest) = data.split_at_checked(end + 2)?;
        if s[end] != b'"' || s[end + 1] != b';' {
            return None;
        }

        self.data = rest;
        Some(PhpBstr::new(&s[6..end]))
    }

    /// Try to consume an end token (`}`). Returns true if consumed.
    #[inline]
    pub(crate) fn try_read_end(&mut self) -> bool {
        match self.data {
            [b'}', rest @ ..] => {
                self.data = rest;
                true
            }
            _ => false,
        }
    }

    /// Try to read the next token as an integer.
    #[inline]
    pub(crate) fn try_read_i64(&mut self) -> Option<i64> {
        let data = self.data;
        match data {
            [b'i', b':', rest @ ..] => {
                let (int, data) = to_i64(rest).ok()?;
                self.data = data;
                Some(int)
            }
            _ => None,
        }
    }

    /// Try to read the next token as a float.
    #[inline]
    pub(crate) fn try_read_f64(&mut self) -> Option<f64> {
        let data = self.data;
        match data {
            [b'd', b':', rest @ ..] => {
                let (float, len) = fast_float2::parse_partial(rest).ok()?;
                let (tail, rest) = rest[len..].split_first()?;
                if tail != &b';' {
                    return None;
                }

                self.data = rest;
                Some(float)
            }
            _ => None,
        }
    }

    /// Try to read the next token as an array header.
    #[inline]
    pub(crate) fn try_read_seq_start(&mut self) -> Option<u32> {
        let data = self.data;
        match data {
            [b'a', b':', rest @ ..] => {
                let (elements, rest) = read_u32(rest, b':').ok()?;
                match rest {
                    [b'{', rest @ ..] => {
                        self.data = rest;
                        Some(elements)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    #[cold]
    fn map_error(&self, error: ScalarError) -> Error {
        match error {
            ScalarError::MissingQuotes => (ErrorKind::MissingQuotes {
                position: self.position(),
            })
            .into(),
            ScalarError::Empty => (ErrorKind::Empty {
                position: self.position(),
            })
            .into(),
            ScalarError::Overflow => (ErrorKind::Overflow {
                position: self.position(),
            })
            .into(),
            ScalarError::Invalid => (ErrorKind::InvalidNumber {
                position: self.position(),
            })
            .into(),
            ScalarError::Eof => ErrorKind::Eof.into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
enum ScalarError {
    MissingQuotes,
    Empty,
    Overflow,
    Invalid,
    Eof,
}

#[inline]
fn read_str(data: &[u8]) -> Result<(PhpBstr<'_>, &[u8]), ScalarError> {
    let (len, data) = read_u32(data, b':')?;
    let len = len as usize;
    let Some((contents, rest)) = data.split_at_checked(len + 2) else {
        return Err(ScalarError::Eof);
    };

    match contents {
        [b'"', contents @ .., b'"'] => Ok((PhpBstr::new(contents), rest)),
        _ => Err(ScalarError::MissingQuotes),
    }
}

#[inline]
fn read_u32(mut data: &[u8], delimiter: u8) -> Result<(u32, &[u8]), ScalarError> {
    let mut result = 0u64;
    let mut digits = 0usize;
    while let Some((&c, rest)) = data.split_first() {
        if c.is_ascii_digit() {
            result = result.wrapping_mul(10);
            result = result.wrapping_add(u64::from(c - b'0'));
            data = rest;
            digits += 1;
        } else if c == delimiter {
            if digits == 0 {
                return Err(ScalarError::Empty);
            }

            // u32::MAX is 10 digits; any 11+ digit number overflows.
            // For ≤10 digits, the value fits in u64 without wrapping, so
            // comparing against u32::MAX is sufficient.
            if digits > 10 || result > u64::from(u32::MAX) {
                return Err(ScalarError::Overflow);
            }
            return Ok((result as u32, rest));
        } else {
            return Err(ScalarError::Invalid);
        }
    }

    Err(ScalarError::Eof)
}

#[inline]
fn to_i64(d: &[u8]) -> Result<(i64, &[u8]), ScalarError> {
    let Some((&c, mut data)) = d.split_first() else {
        return Err(ScalarError::Empty);
    };

    let negative = c == b'-';
    let mut digits = 0usize;
    let mut result = if c.is_ascii_digit() {
        digits = 1;
        u64::from(c - b'0')
    } else if c == b'-' {
        0
    } else {
        return Err(ScalarError::Invalid);
    };

    while let Some((&c, rest)) = data.split_first() {
        if c.is_ascii_digit() {
            result = result.wrapping_mul(10);
            result = result.wrapping_add(u64::from(c - b'0'));
            data = rest;
            digits += 1;
        } else if c == b';' {
            if digits == 0 {
                return Err(ScalarError::Empty);
            }

            // i64::MAX is 19 digits; u64::MAX is 20 digits, so wrapping only
            // occurs at 20+ digits. Guard against that before the value check.
            if digits > 19 || result > (i64::MAX as u64) + u64::from(negative) {
                return Err(ScalarError::Overflow);
            }

            let sign: i64 = if negative { -1 } else { 1 };
            return Ok((sign.wrapping_mul(result as i64), rest));
        } else {
            return Err(ScalarError::Invalid);
        }
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
                parser.position()
            );

            // Check if the token matches what we expect
            let expected_token = &expected_tokens[token_index];
            assert_eq!(
                &actual_token,
                expected_token,
                "Token mismatch at position {}: expected {:?}, got {:?} at position {}",
                token_index,
                expected_token,
                actual_token,
                parser.position()
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
    #[case("i:9223372036854775807;", PhpToken::Integer(i64::MAX))]
    #[case("i:-9223372036854775808;", PhpToken::Integer(i64::MIN))]
    #[case("i:-9223372036854775807;", PhpToken::Integer(i64::MIN + 1))]
    #[case("i:2147483648;", PhpToken::Integer(2147483648))] // i32::MAX + 1
    #[case("i:-2147483649;", PhpToken::Integer(-2147483649))] // i32::MIN - 1
    fn test_parse_integer(#[case] input: &str, #[case] expected: PhpToken<'_>) {
        let mut parser = PhpParser::new(input.as_bytes());
        assert_eq!(parser.next_token().unwrap(), Some(expected));
    }

    #[rstest]
    #[case("d:3.33;", PhpToken::Float(3.33))]
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
    #[case(b"s:0:\"\";i:-3;i:9;", b"", 7, -3)]
    #[case(b"s:5:\"hello\";i:7;", b"hello", 12, 7)]
    #[case(b"s:10:\"0123456789\";i:7;", b"0123456789", 18, 7)]
    fn test_try_read_str(
        #[case] input: &[u8],
        #[case] expected: &[u8],
        #[case] expected_position: usize,
        #[case] next_integer: i64,
    ) {
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.try_read_str(), Some(PhpBstr::new(expected)));
        assert_eq!(parser.position(), expected_position);
        assert_eq!(
            parser.read_token().unwrap(),
            PhpToken::Integer(next_integer)
        );
    }

    #[test]
    fn test_try_read_str_returns_none_without_consuming_non_string_token() {
        let mut parser = PhpParser::new(b"i:42;");

        assert_eq!(parser.try_read_str(), None);
        assert_eq!(parser.position(), 0);
        assert_eq!(parser.read_token().unwrap(), PhpToken::Integer(42));
    }

    #[rstest]
    #[case(b"i:-3;s:2:\"ok\";", -3, 5, PhpToken::String(PhpBstr::new(b"ok")))]
    #[case(b"i:42;i:7;", 42, 5, PhpToken::Integer(7))]
    fn test_try_read_i64(
        #[case] input: &[u8],
        #[case] expected: i64,
        #[case] expected_position: usize,
        #[case] next_token: PhpToken<'_>,
    ) {
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.try_read_i64(), Some(expected));
        assert_eq!(parser.position(), expected_position);
        assert_eq!(parser.read_token().unwrap(), next_token);
    }

    #[test]
    fn test_try_read_i64_returns_none_without_consuming_non_integer_token() {
        let mut parser = PhpParser::new(b"s:2:\"ok\";");

        assert_eq!(parser.try_read_i64(), None);
        assert_eq!(parser.position(), 0);
        assert_eq!(
            parser.read_token().unwrap(),
            PhpToken::String(PhpBstr::new(b"ok"))
        );
    }

    #[rstest]
    #[case(b"d:3.33;i:7;", 3.33, 7, 7)]
    #[case(b"d:-0.5;i:7;", -0.5, 7, 7)]
    #[case(b"d:1.0E+25;i:7;", 1.0E25, 10, 7)]
    fn test_try_read_f64(
        #[case] input: &[u8],
        #[case] expected: f64,
        #[case] expected_position: usize,
        #[case] next_integer: i64,
    ) {
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.try_read_f64(), Some(expected));
        assert_eq!(parser.position(), expected_position);
        assert_eq!(
            parser.read_token().unwrap(),
            PhpToken::Integer(next_integer)
        );
    }

    #[test]
    fn test_try_read_f64_returns_none_without_consuming_non_float_token() {
        let mut parser = PhpParser::new(b"i:42;");

        assert_eq!(parser.try_read_f64(), None);
        assert_eq!(parser.position(), 0);
        assert_eq!(parser.read_token().unwrap(), PhpToken::Integer(42));
    }

    #[rstest]
    #[case(b"a:0:{}i:7;", 0, 5, PhpToken::End)]
    #[case(
        b"a:3:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";i:2;s:3:\"baz\";}",
        3,
        5,
        PhpToken::Integer(0)
    )]
    #[case(b"a:12:{}i:7;", 12, 6, PhpToken::End)]
    fn test_try_read_seq_start(
        #[case] input: &[u8],
        #[case] expected_elements: u32,
        #[case] expected_position: usize,
        #[case] next_token: PhpToken<'_>,
    ) {
        let mut parser = PhpParser::new(input);

        assert_eq!(parser.try_read_seq_start(), Some(expected_elements));
        assert_eq!(parser.position(), expected_position);
        assert_eq!(parser.read_token().unwrap(), next_token);
    }

    #[test]
    fn test_try_read_seq_start_returns_none_without_consuming_non_array_token() {
        let mut parser = PhpParser::new(b"i:42;");

        assert_eq!(parser.try_read_seq_start(), None);
        assert_eq!(parser.position(), 0);
        assert_eq!(parser.read_token().unwrap(), PhpToken::Integer(42));
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

        let prop = bstr.to_property();
        assert_eq!((prop.to_str().unwrap(), prop.visibility()), expected);
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
    fn test_parse_custom_object() {
        let input = b"C:5:\"Test2\":6:{foobar}";
        let expected = [PhpToken::CustomObject {
            class: PhpBstr::new(b"Test2"),
            payload: PhpBstr::new(b"foobar"),
        }];
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

    #[test]
    fn test_invalid_token_reports_offending_position() {
        let mut parser = PhpParser::new(b"x:invalid;");
        let error = parser.next_token().unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::UnexpectedByte { position, .. } if *position == 0
        ));
    }

    #[test]
    fn test_invalid_token_peek_reports_offending_position() {
        let mut parser = PhpParser::new(b"x:invalid;");
        let error = parser.peek_token().unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::UnexpectedByte { position, .. } if *position == 0
        ));
    }

    #[test]
    fn test_leading_newline_is_rejected() {
        let mut parser = PhpParser::new(b"\ni:42;");
        let error = parser.next_token().unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::UnexpectedByte {
                found: b'\n',
                position: 0
            }
        ));
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
    #[case(b"i:-;")]
    #[case(b"i:--1;")]
    #[case(b"i:1a;")]
    #[case(b"i: 42;")]
    #[case(b"i:+42;")]
    #[case(b"i:9223372036854775808;")] // i64::MAX + 1
    #[case(b"i:9999999999999999999;")] // Definitely too big for i64
    #[case(b"i:-9223372036854775809;")] // i64::MIN - 1
    #[case(b"i:-9999999999999999999;")] // Definitely too small for i64
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
    #[case(b"s::\"\";")]
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
    #[case(b"s:10:\"hello\";")]
    #[case(b"s:1000:\"hello\";")]
    fn test_truncated_string_reports_eof(#[case] input: &[u8]) {
        let mut parser = PhpParser::new(input);
        let error = parser.next_token().unwrap_err();
        assert!(matches!(error.kind(), ErrorKind::Eof));
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
    #[case(b"a::{")]
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
    #[case(b"O:3:\"Foo\"::{")]
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
    #[case(b"r:-;")]
    #[case(b"r:xyz;")]
    #[case(b"r:9223372036854775808;")]
    #[case(b"r:-9223372036854775809;")]
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

    #[test]
    fn test_readme() -> Result<(), Box<dyn std::error::Error>> {
        let serialized = b"O:7:\"Example\":5:{s:4:\"name\";s:8:\"John Doe\";s:12:\"\0Example\0age\";i:42;s:11:\"\0*\0isActive\";b:1;s:6:\"scores\";a:3:{i:0;d:95.5;i:1;d:88.0;i:2;d:92.3;}s:8:\"metadata\";a:2:{s:2:\"id\";i:12345;s:4:\"tags\";a:3:{i:0;s:3:\"php\";i:1;s:4:\"rust\";i:2;s:13:\"serialization\";}}}";
        let mut parser = PhpParser::new(&serialized[..]);

        assert_eq!(
            parser.read_token()?,
            PhpToken::Object {
                class: PhpBstr::new(b"Example"),
                properties: 5
            }
        );

        let PhpToken::String(prop) = parser.read_token()? else {
            panic!("Expected a string token");
        };

        assert_eq!(prop, PhpBstr::new(b"name"));
        let prop = prop.to_property();
        assert_eq!(prop.to_str()?, "name");
        assert_eq!(prop.visibility(), PhpVisibility::Public);

        Ok(())
    }
}
