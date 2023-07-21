#![no_main]
use libfuzzer_sys::fuzz_target;
use memedb_core::{read_tags, write_tags};
use std::io::Cursor;

fuzz_target!(|data: (Vec<u8>, Vec<String>)| {
    if let Ok(Some(_)) = read_tags(&mut Cursor::new(&data.0)) {
        let mut dest = Vec::new();
        if let Ok(Some(_)) = write_tags(&mut Cursor::new(data.0), &mut dest, data.1.clone()) {
            assert_eq!(read_tags(&mut Cursor::new(dest)).unwrap().unwrap(), data.1);
        }
    }
});
