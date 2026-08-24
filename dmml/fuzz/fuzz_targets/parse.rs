//! Fuzzes `dmml::parse` against arbitrary byte input. `parse`'s only
//! documented invariant is that it never panics -- every rejection is a
//! `ParseError`, never a crash -- so this target's whole job is to feed
//! it garbage and let libFuzzer's sanitizers catch any violation of that
//! (panic, out-of-bounds slice, integer overflow in debug, etc.).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = dmml::parse(s);
    }
});
