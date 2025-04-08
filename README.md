# PHPserz

A Rust crate for parsing and deserializing the PHP serialization format[^wiki].

[^wiki]: https://en.wikipedia.org/wiki/PHP_serialization_format

## Features

- Support for PHP objects with public, protected, and private members
- Fast. Exceeding 1 GiB/s in application benchmarks
- Zero allocation and zero copy parsing

## Quick start

To start, below is an example of generating PHP serialized output

```php
<?php
class Example {
    public $name = "John Doe";
    private $age = 42;
    protected $isActive = true;
    public $scores = [95.5, 88.0, 92.3];
    public $metadata = [
        "id" => 12345,
        "tags" => ["php", "rust", "serialization"]
    ];
}

$example = new Example();
$serialized = serialize($example);
echo $serialized;
```

The output will be:

```plain,ignore
O:7:"Example":5:{s:4:"name";s:8:"John Doe";s:12:"\0Example\0age";i:42;s:11:"\0*\0isActive";b:1;s:6:"scores";a:3:{i:0;d:95.5;i:1;d:88.0;i:2;d:92.3;}s:8:"metadata";a:2:{s:2:"id";i:12345;s:4:"tags";a:3:{i:0;s:3:"php";i:1;s:4:"rust";i:2;s:13:"serialization";}}}
```

### Deserialization

PHPserz supports ergonomic serde deserialization:

```rust
#[cfg(feature = "serde")] {
use phpserz::PhpDeserializer;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, PartialEq)]
struct Example {
    name: String,
    age: i32,
    #[serde(rename = "isActive")]
    is_active: bool,
    scores: BTreeMap<u32, f64>,
    metadata: Metadata,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Metadata {
    id: i32,
    tags: BTreeMap<u32, String>,
}

let serialized = b"O:7:\"Example\":5:{s:4:\"name\";s:8:\"John Doe\";s:12:\"\0Example\0age\";i:42;s:11:\"\0*\0isActive\";b:1;s:6:\"scores\";a:3:{i:0;d:95.5;i:1;d:88.0;i:2;d:92.3;}s:8:\"metadata\";a:2:{s:2:\"id\";i:12345;s:4:\"tags\";a:3:{i:0;s:3:\"php\";i:1;s:4:\"rust\";i:2;s:13:\"serialization\";}}}";

let mut deserializer = PhpDeserializer::new(serialized);
let example = Example::deserialize(&mut deserializer).unwrap();

assert_eq!(
    example,
    Example {
        name: "John Doe".to_string(),
        age: 42,
        is_active: true,
        scores: BTreeMap::from([
            (0, 95.5),
            (1, 88.0),
            (2, 92.3),
        ]),
        metadata: Metadata {
            id: 12345,
            tags: BTreeMap::from([
                (0, "php".to_string()),
                (1, "rust".to_string()),
                (2, "serialization".to_string())
            ]),
        }
    }
);
}
```

### Token Parsing

One can go one level lower and drive the parser manually to better inspect the data.

```rust
use phpserz::{Error, PhpParser, PhpToken, PhpBstr, PhpVisibility};

let serialized = b"O:7:\"Example\":5:{s:4:\"name\";s:8:\"John Doe\";s:12:\"\0Example\0age\";i:42;s:11:\"\0*\0isActive\";b:1;s:6:\"scores\";a:3:{i:0;d:95.5;i:1;d:88.0;i:2;d:92.3;}s:8:\"metadata\";a:2:{s:2:\"id\";i:12345;s:4:\"tags\";a:3:{i:0;s:3:\"php\";i:1;s:4:\"rust\";i:2;s:13:\"serialization\";}}}";

let mut parser = PhpParser::new(&serialized[..]);

assert_eq!(
    parser.read_token().unwrap(),
    PhpToken::Object {
        class: PhpBstr::new(b"Example"),
        properties: 5
    }
);

let PhpToken::String(prop) = parser.read_token().unwrap() else {
    panic!("Expected a string token");
};

assert_eq!(prop, PhpBstr::new(b"name"));
let (name, visibility) = prop.to_property().unwrap();
assert_eq!(name, "name");
assert_eq!(visibility, PhpVisibility::Public);

// ...
```
