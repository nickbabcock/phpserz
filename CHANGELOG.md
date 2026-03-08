## v0.3.0 - March 8th 2026

- Breaking: `PhpBstr::to_property()` now returns `PhpProperty` instead of `Result<(&str, PhpVisibility), Error>`, and `PhpProperty` is now public
- Breaking: `PhpToken::Reference(i64)` became `PhpToken::Reference { id, kind }` and `PhpTokenKind::Reference` became `PhpTokenKind::Reference(PhpReferenceKind)`
- Changed: Deserializing PHP arrays as Rust sequences now requires dense integer keys starting at `0`; sparse or associative arrays now error
- Changed: `deserialize_any` now exposes PHP arrays as maps instead of sequences
- Changed: Parser validation and diagnostics are stricter by rejecting leading newlines and empty numeric fields, reporting EOF for truncated strings, and improving unexpected-byte positions
- Changed: `deserialize_str` no longer falls back to borrowed bytes for invalid UTF-8; callers that need raw bytes should use `deserialize_bytes`
- Added: Support for Serde externally tagged enum wrapper forms encoded as single-entry PHP arrays or one-property PHP objects
- Added: Support for PHP custom `C` object tokens
- Added: Support for uppercase `R` alias references in addition to lowercase `r` repeated references
- Perf: 2x faster deserialization with speculative parsing based on deserialization hint

## v0.2.2 - October 14th 2025

- Remove dependence on serde derive feature

## v0.2.1 - May 10th 2025

- Add borrowed string and bytes deserialization

## v0.2.0 - April 14th 2025

- Add basic enum deserialization support
- Add ability to convert parsers to and from deserializers
- Fix deserializer visiting bytes even when a string is requested

## v0.1.0 - April 8th 2025

- Initial version
