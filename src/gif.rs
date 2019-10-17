#![allow(clippy::unreadable_literal)]
use crate::error::Error;
use crate::reader::{IoResult, Reader};
use crate::TagSet;
use log::{debug, info};
use std::io::Error as IoError;

pub struct GifReader {}

pub const SIGNATURE: &[u8] = b"GIF89a";

impl Reader for GifReader {
    fn read_tags(file: &mut impl Iterator<Item = IoResult>) -> Result<TagSet, Error> {
        let bytes = file.collect::<Result<Vec<u8>, IoError>>()?;

        let (mut i, found) = GifReader::find_tags(&bytes)?;
        let mut tags = TagSet::new();
        if !found {
            Ok(tags)
        } else {
            loop {
                if bytes[i] == 0 {
                    return Ok(tags);
                }
                let sub_block_size = bytes[i] as usize;
                i += 1;
                tags.insert(String::from_utf8_lossy(&bytes[i..i + sub_block_size]).to_string());
                i += sub_block_size;
            }
        }
    }

    fn write_tags(
        file: &mut impl Iterator<Item = IoResult>,
        tags: &TagSet,
    ) -> Result<Vec<u8>, Error> {
        let mut bytes: Vec<u8> = SIGNATURE
            .iter()
            .copied()
            .map(Ok)
            .chain(file)
            .collect::<Result<_, IoError>>()?;

        let mut tags: Vec<String> = tags.iter().cloned().collect();
        tags.sort_unstable();
        let mut tag_bytes = Vec::new();
        for tag in &mut tags {
            tag_bytes.push(tag.len() as u8);
            tag_bytes.append(&mut tag.as_bytes().to_vec());
        }
        tag_bytes.push(0);

        let (i, found) = GifReader::find_tags(&bytes[SIGNATURE.len()..])?; // Skip signature, but find tags as if it didn't existed
        let mut i = i + SIGNATURE.len(); // add SIGNATURE.len() to i, to include SIGNATURE
        if !found {
            info!("Appending bytes at {:X}", i);
            let mut insert_bytes = b"\x21\xFF\x0BMEMETAGS1.0".to_vec();
            insert_bytes.append(&mut tag_bytes);
            bytes.splice(i..i, insert_bytes);
            Ok(bytes)
        } else {
            info!("Inserting bytes at {:X}", i);
            let start = i;
            loop {
                if bytes[i] == 0 {
                    bytes.splice(start..i, tag_bytes);
                    return Ok(bytes);
                }
                let sub_block_size = bytes[i] as usize;
                i += sub_block_size + 1;
            }
        }
    }
}

impl GifReader {
    fn get_color_table_size(byte: u8) -> usize {
        if byte >> 7 & 1 == 0 {
            0
        } else {
            let size = byte & 0b00000111;
            3 << (size + 1)
        }
    }

    fn find_tags(bytes: &[u8]) -> Result<(usize, bool), Error> {
        let mut i: usize = 0;

        // Get info from descriptor
        let color_table_size = GifReader::get_color_table_size(bytes[i + 4]);
        i += 7;

        // Skip color table
        i += color_table_size;

        loop {
            debug!("Reading block {}", bytes[i]);
            match bytes[i] {
                // Trailer, signifies end of file
                0x3B => {
                    info!("Reached end and no tags in sight");
                    return Ok((i, false));
                }
                // Extension block
                0x21 => {
                    let label = bytes[i + 1];
                    let size = bytes[i + 2] as usize;
                    let data = &bytes[i + 3..i + 3 + size];
                    i += 3 + size;
                    debug!(
                        "Extension block label: {}, data: {}",
                        label,
                        String::from_utf8_lossy(data)
                    );

                    if label == 0xFF && data == b"MEMETAGS1.0" {
                        info!("Found tags at {:X}", i);
                        return Ok((i, true));
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
                _ => return Err(Error::Parser),
            };
        }
    }
}

reader_tests!(GifReader, "gif");
