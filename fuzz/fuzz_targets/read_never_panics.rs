#![no_main]
use libfuzzer_sys::fuzz_target;
use memedb_core::read_tags;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let _ = read_tags(&mut Cursor::new(data));
});
