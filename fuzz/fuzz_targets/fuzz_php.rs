#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut parser = phpserz::PhpParser::new(data);
    while let Ok(Some(token)) = parser.next_token() {
        let phpserz::PhpToken::String(bstr) = token else {
            continue;
        };

        let _ = bstr.to_property();
    }
});
