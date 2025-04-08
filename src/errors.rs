/// An PHPserz error.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    /// Get the kind of error.
    #[must_use]
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Get the position of the error.
    #[must_use]
    pub const fn position(&self) -> Option<usize> {
        match &self.kind {
            ErrorKind::MismatchByte { position, .. }
            | ErrorKind::UnexpectedByte { position, .. }
            | ErrorKind::Empty { position }
            | ErrorKind::MissingQuotes { position }
            | ErrorKind::StringTooLong { position }
            | ErrorKind::InvalidNumber { position }
            | ErrorKind::Overflow { position } => Some(*position),
            ErrorKind::Deserialize { position, .. } => *position,
            ErrorKind::Eof | ErrorKind::Utf8(_) => None,
        }
    }
}

/// The kind of error that can occur when working with PHP serialized data.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    Eof,
    MismatchByte {
        expected: u8,
        found: u8,
        position: usize,
    },
    UnexpectedByte {
        found: u8,
        position: usize,
    },
    Utf8(std::str::Utf8Error),
    Deserialize {
        message: String,
        position: Option<usize>,
    },
    Empty {
        position: usize,
    },
    MissingQuotes {
        position: usize,
    },
    StringTooLong {
        position: usize,
    },
    InvalidNumber {
        position: usize,
    },
    Overflow {
        position: usize,
    },
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Empty { .. }
            | ErrorKind::MismatchByte { .. }
            | ErrorKind::UnexpectedByte { .. }
            | ErrorKind::Eof
            | ErrorKind::Deserialize { .. }
            | ErrorKind::StringTooLong { .. }
            | ErrorKind::InvalidNumber { .. }
            | ErrorKind::Overflow { .. }
            | ErrorKind::MissingQuotes { .. } => None,
            ErrorKind::Utf8(err) => Some(err),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::MismatchByte {
                expected,
                found,
                position,
            } => {
                if found.is_ascii_alphanumeric() {
                    write!(
                        f,
                        "Expected byte '{}', found '{}' at position: {}",
                        *expected as char, *found as char, position
                    )
                } else {
                    write!(
                        f,
                        "Expected byte '{}', found 0x{:02x} at position: {}",
                        *expected as char, found, position
                    )
                }
            }
            ErrorKind::UnexpectedByte { found, position } => {
                if found.is_ascii_alphanumeric() {
                    write!(
                        f,
                        "Unexpected byte '{}' at position: {}",
                        *found as char, position
                    )
                } else {
                    write!(f, "Unexpected byte 0x{found:02x} at position: {position}")
                }
            }
            ErrorKind::Eof => write!(f, "Unexpected end of data"),
            ErrorKind::Deserialize { message, position } => {
                if let Some(pos) = position {
                    write!(f, "Deserialization error: {message} at position: {pos}")
                } else {
                    write!(f, "Deserialization error: {message}")
                }
            }
            ErrorKind::Empty { position } => {
                write!(f, "Unable to decode empty data at position: {position}")
            }
            ErrorKind::MissingQuotes { position } => {
                write!(f, "Missing quotes in string at position: {position}")
            }
            ErrorKind::StringTooLong { position } => {
                write!(f, "String is too long at position: {position}")
            }
            ErrorKind::InvalidNumber { position } => {
                write!(f, "Invalid number at position: {position}")
            }
            ErrorKind::Overflow { position } => write!(f, "Overflow at position: {position}"),
            ErrorKind::Utf8(err) => write!(f, "UTF-8 conversion error: {err}"),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::from(ErrorKind::Deserialize {
            message: msg.to_string(),
            position: None,
        })
    }
}
