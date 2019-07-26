use crate::error::Error;
use crate::reader::Reader;
use std::collections::HashSet;
use std::io::Read;

pub struct GifReader {}

impl GifReader {
    fn get_color_table_size(byte: u8) -> usize {
        if byte >> 7 & 1 == 0 {
            0
        } else {
            let size = byte & 0b00000111;
            3 * 1 << (size + 1)
        }
    }
}

impl Reader for GifReader {
    fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let mut i: usize = 0;

        // Verify signature
        if bytes[0..6] != *b"GIF89a" {
            return Err(Error::UnknownFormat);
        }
        i += 6;

        // Get info from descriptor
        let color_table_size = GifReader::get_color_table_size(bytes[10]);
        i += 7;

        // Skip color table
        i += color_table_size;

        loop {
            match bytes[i] {
                // Trailer, signifies end of file
                0x3B => return Err(Error::UnexpectedEOF),
                // Extension block
                0x21 => {
                    let label = bytes[i + 1];
                    let size = bytes[i + 2] as usize;
                    let data = &bytes[i + 3..i + 3 + size];
                    i += 3 + size;

                    if label == 0xFF && data == b"MEMETAGS1.0" {
                        let mut tags = HashSet::new();
                        loop {
                            if bytes[i] == 0 {
                                return Ok(tags);
                            }
                            let sub_block_size = bytes[i] as usize;
                            tags.insert(
                                String::from_utf8_lossy(&bytes[i..i + sub_block_size]).to_string(),
                            );
                            i += sub_block_size + 1;
                        }
                    }

                    loop {
                        if bytes[i] == 0 {
                            i += 1;
                            break;
                        }
                        let sub_block_size = bytes[i] as usize;
                        i += sub_block_size + 1;
                    }
                }
                // Image Block
                0x2C => {
                    let color_table_size = GifReader::get_color_table_size(bytes[i + 9]);
                    i += 10;

                    i += color_table_size;

                    // Loop through sub-blocks
                    i += 1;
                    loop {
                        if bytes[i] == 0 {
                            i += 1;
                            break;
                        }
                        let sub_block_size = bytes[i] as usize;
                        i += sub_block_size + 1;
                    }
                }
                _ => return Err(Error::UnknownFormat),
            };
        }
    }

    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_read_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        // mem::discriminant magic is used to compare enums without having to implement PartialEq
        assert_eq!(
            std::mem::discriminant(&GifReader::read_tags(&mut file).unwrap_err()),
            std::mem::discriminant(&Error::UnknownFormat)
        );
    }

    #[test]
    fn test_read_empty() {
        let mut file = File::open("tests/empty.gif").unwrap();
        assert_eq!(GifReader::read_tags(&mut file).unwrap(), HashSet::new());
    }
}
