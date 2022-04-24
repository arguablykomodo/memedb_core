#![no_main]
use libfuzzer_sys::fuzz_target;
use memedb_core::{are_tags_valid, read_tags, write_tags};
use std::collections::HashSet;
use std::io::Cursor;

fuzz_target!(|data: (Vec<u8>, HashSet<String>)| {
    if are_tags_valid(&data.1) {
        if let Ok(Some(_)) = read_tags(&mut Cursor::new(&data.0)) {
            let mut new_data = Vec::new();
            let result = write_tags(&mut Cursor::new(data.0), &mut new_data, data.1.clone());
            if let Ok(Some(_)) = result {
                assert_eq!(read_tags(&mut Cursor::new(new_data)).unwrap().unwrap(), data.1);
            }
        }
    }
});
