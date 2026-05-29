#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg(feature = "serde")]
mod de;
mod errors;
mod parser;
#[cfg(feature = "serde")]
mod ser;

#[cfg(feature = "serde")]
pub use de::PhpDeserializer;
pub use errors::{Error, ErrorKind};
pub use parser::{
    PhpBstr, PhpParser, PhpProperty, PhpReferenceKind, PhpToken, PhpTokenKind, PhpVisibility,
};
#[cfg(feature = "serde")]
pub use ser::{PhpSerializer, StructStyle};
