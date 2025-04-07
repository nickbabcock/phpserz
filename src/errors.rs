use std::fmt;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Io(std::io::Error),
    PhpParse(PhpParseError),
    Utf8(std::str::Utf8Error, usize),
    ParseInt(std::num::ParseIntError, usize),
    ParseFloat(std::num::ParseFloatError, usize),
    Deserialize(String),
}

#[derive(Debug)]
pub struct PhpParseError {
    pub message: String,
    pub position: usize,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(err) => Some(err),
            ErrorKind::Deserialize(_) => None,
            ErrorKind::PhpParse(_) => None,
            ErrorKind::Utf8(err, _) => Some(err),
            ErrorKind::ParseInt(err, _) => Some(err),
            ErrorKind::ParseFloat(err, _) => Some(err),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Io(err) => write!(f, "IO error: {}", err),
            ErrorKind::Deserialize(err) => write!(f, "Deserialization error: {}", err),
            ErrorKind::PhpParse(err) => write!(
                f,
                "PHP parse error at byte position {}: {}",
                err.position, err.message
            ),
            ErrorKind::Utf8(err, pos) => write!(
                f,
                "UTF-8 conversion error at byte position {}: {}",
                pos, err
            ),
            ErrorKind::ParseInt(err, pos) => {
                write!(f, "Integer parse error at byte position {}: {}", pos, err)
            }
            ErrorKind::ParseFloat(err, pos) => {
                write!(f, "Float parse error at byte position {}: {}", pos, err)
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error {
            kind: ErrorKind::Io(err),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { kind }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::from(ErrorKind::Deserialize(msg.to_string()))
    }
}
