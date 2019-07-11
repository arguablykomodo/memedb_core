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

        // Get info we need from Logical Screen Descriptor
        let has_color_table = bytes[10] >> 7 == 1;
        let color_table_size =
            2 << (bytes[10] & 1 + bytes[10] >> 1 & 1 * 2 + bytes[10] >> 2 & 1 * 4 + 1);
        i += 7;

        unimplemented!();
    }
    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
        unimplemented!();
    }
}
