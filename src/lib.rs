#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg(feature = "serde")]
mod de;
mod errors;
mod parser;

#[cfg(feature = "serde")]
pub use de::PhpDeserializer;
pub use errors::{Error, ErrorKind};
pub use parser::{PhpBstr, PhpParser, PhpToken, PhpTokenKind, PhpVisibility};
