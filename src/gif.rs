use crate::error::Error;
use crate::reader::Reader;
use std::collections::HashSet;
use std::io::Read;

pub struct GifReader {}

impl Reader for GifReader {
    fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let mut i = 0;

        // Verify signature
        if bytes[0..5] != *b"GIF89a" {
            return Err(Error::UnknownFormat);
        }
        i += 6;

        unimplemented!();
    }
    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
        unimplemented!();
    }
}
