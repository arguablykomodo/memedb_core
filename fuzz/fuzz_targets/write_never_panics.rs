#![no_main]
use libfuzzer_sys::fuzz_target;
use memedb_core::write_tags;
use std::io::{sink, Cursor};

fuzz_target!(|data: (Vec<u8>, Vec<String>)| {
    let _ = write_tags(&mut Cursor::new(data.0), &mut sink(), data.1);
});
