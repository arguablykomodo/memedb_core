use crate::error::Error;
use crate::reader::Reader;
use std::collections::HashSet;
use std::io::Read;

pub struct GifReader {}

impl GifReader {
    fn get_color_table_size(byte: u8) -> usize {
        let mut size = 0;
        for i in 0..2 {
            size += byte >> i & 1 << i;
        }
        3 * 2 << (size + 1)
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
                0x3B => {
                    break;
                }
                // Extension block
                0x21 => {
                    println!("Extension block: {:X}", bytes[i + 1]);
                    let data_size = bytes[i + 2] as usize;
                    i += 3 + data_size + 1;
                }
                // Image Block
                0x2C => {
                    let color_table_size = GifReader::get_color_table_size(bytes[i + 9]);
                    i += 10 + color_table_size + 1;

                    // Loop through sub-blocks
                    loop {
                        if bytes[i] == 0 {
                            break;
                        }
                        let sub_block_size = bytes[i] as usize;
                        i += sub_block_size;
                    }
                }
                _ => {}
            };
        }

        unimplemented!();
    }
    fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
        unimplemented!();
    }
}
