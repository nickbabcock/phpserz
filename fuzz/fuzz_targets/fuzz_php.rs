#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut parser = phpserz::PhpParser::new(data);

    let mut storage = Vec::new();
    while let Ok(Some(_)) = parser.next_token(&mut storage) {

    }
});
