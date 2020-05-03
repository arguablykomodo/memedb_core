use crate::{
    error::{Error, Result},
    TagSet,
};
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = b"GIF89a";

// Info comes from https://www.matthewflickinger.com/lab/whatsinagif/bits_and_bytes.asp
// Incomprehensible TL;DR is as follows:
// Logical Screen Descriptor: 7 bytes
//   5th byte is a packed byte:
//     1st bit: global color table flag (0=no table)
//     last 3 bits: global color table size ($S)
// Optional Global Color Table (marked by flag in Logical Screen Descriptor): 3*2^($S+1) bytes
// Sub-blocks: [$N, $N bytes] <- repeat until $N == 0
// Graphics Control Extension: 0x21, 0xF9, $N, $N bytes, then sub-blocks
// Image Descriptor: 0x2C, 8 bytes, packed byte:
//   1st bit: local color table flag (0=no table)
//   last 3 bits: local color table size ($S)
// Optional Local Color Table (marked by flag in Image Descriptor): 3*2^($S+1) bytes
// Image Data: 1 byte plus sub-blocks
// Plaintext Extension: 0x21, 0x01, $N, $N bytes, then sub-blocks
// Application Extension: 0x21, 0xFF, $N(0x0B), $N bytes, then sub-blocks
// Comment Extension: 0x21, 0xFE, then sub-blocks
// Trailer: 0x3B, EOF
// Layout:
// Logical Screen Descriptor, Global Color Table?, (
//   Application Extension |
//   Comment Extension |
//   (Graphics Control Extension?, (
//     (Image Descriptor, Local Color Table?, Image Data) |
//     Plaintext Extension
//   ))
// )*, Trailer
// tags are stored as sub-blocks inside MEMETAGS1.0 Application Extension

const IDENTIFIER: &[u8; 11] = b"MEMETAGS1.0";

fn skip_sub_blocks(src: &mut (impl Read + Seek)) -> Result<()> {
    loop {
        let sub_block_length = read_bytes!(src, 1);
        if sub_block_length == 0 {
            return Ok(());
        } else {
            skip_bytes!(src, sub_block_length as i64)?;
        }
    }
}

fn write_sub_blocks(src: &mut (impl Read + Seek), dest: &mut impl Write) -> Result<()> {
    loop {
        let sub_block_length = read_bytes!(src, 1);
        dest.write_all(&[sub_block_length])?;
        if sub_block_length == 0 {
            return Ok(());
        } else {
            dest.write_all(&read_bytes!(src, sub_block_length as usize)[..])?;
        }
    }
}

fn get_color_table_size(byte: u8) -> u16 {
    let has_global_color_table = byte & 0b10000000;
    if has_global_color_table >> 7 == 1 {
        let packed_size = byte & 0b00000111;
        let size = 3 * 2u16.pow(packed_size as u32 + 1);
        return size;
    } else {
        return 0;
    }
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    let logical_screen_descriptor = read_bytes!(src, 7);
    let color_table_size = get_color_table_size(logical_screen_descriptor[4]);
    skip_bytes!(src, color_table_size as i64)?;

    loop {
        let identifier = read_bytes!(src, 1);
        match identifier {
            // Extension
            0x21 => {
                let extension_type = read_bytes!(src, 1);
                match extension_type {
                    // Application Extension
                    0xFF => {
                        let block_size = read_bytes!(src, 1); // Should always be 11
                        let application_identifier = read_bytes!(src, block_size as usize);
                        if application_identifier == IDENTIFIER {
                            let mut tags = TagSet::new();
                            loop {
                                let tag_length = read_bytes!(src, 1);
                                if tag_length == 0 {
                                    return Ok(tags);
                                } else {
                                    let tag_bytes = read_bytes!(src, tag_length as usize);
                                    tags.insert(std::str::from_utf8(&tag_bytes)?.to_string());
                                }
                            }
                        } else {
                            skip_sub_blocks(src)?;
                        }
                    }
                    // Comment Extension
                    0xFE => skip_sub_blocks(src)?,
                    // Graphics Control Extension and Plaintext Extension
                    0xF9 | 0x01 => {
                        let block_size = read_bytes!(src, 1);
                        skip_bytes!(src, block_size as i64)?;
                        skip_sub_blocks(src)?;
                    }
                    byte => return Err(Error::GifUnknownExtension(byte)),
                }
            }
            // Image descriptor
            0x2C => {
                let data = read_bytes!(src, 9);
                let color_table_size = get_color_table_size(data[8]);
                // Extra byte skipped is LZW Minimum Code Size, i dont know what it is and i dont care
                skip_bytes!(src, color_table_size as i64 + 1)?;
                skip_sub_blocks(src)?;
            }
            // EOF
            0x3B => return Ok(TagSet::new()),
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    dest.write_all(SIGNATURE)?;

    let logical_screen_descriptor = read_bytes!(src, 7);
    dest.write_all(&logical_screen_descriptor)?;
    let color_table_size = get_color_table_size(logical_screen_descriptor[4]);
    dest.write_all(&read_bytes!(src, color_table_size as usize)[..])?;

    loop {
        let identifier = read_bytes!(src, 1);
        match identifier {
            // Extension
            0x21 => {
                let extension_type = read_bytes!(src, 1);
                match extension_type {
                    // Application Extension
                    0xFF => {
                        let block_size = read_bytes!(src, 1); // Should always be 11
                        let application_identifier = read_bytes!(src, block_size as usize);
                        if application_identifier == IDENTIFIER {
                            skip_sub_blocks(src)?;
                        } else {
                            dest.write_all(&[identifier, extension_type])?;
                            dest.write_all(&[block_size])?;
                            dest.write_all(&application_identifier[..])?;
                            write_sub_blocks(src, dest)?;
                        }
                    }
                    // Comment Extension
                    0xFE => {
                        dest.write_all(&[identifier, extension_type])?;
                        write_sub_blocks(src, dest)?;
                    }
                    // Graphics Control Extension and Plaintext Extension
                    0xF9 | 0x01 => {
                        dest.write_all(&[identifier, extension_type])?;
                        let block_size = read_bytes!(src, 1);
                        dest.write_all(&[block_size])?;
                        dest.write_all(&read_bytes!(src, block_size as usize)[..])?;
                        write_sub_blocks(src, dest)?;
                    }
                    byte => return Err(Error::GifUnknownExtension(byte)),
                }
            }
            // Image descriptor
            0x2C => {
                dest.write_all(&[identifier])?;
                let data = read_bytes!(src, 9);
                dest.write_all(&data)?;
                let color_table_size = get_color_table_size(data[8]);
                // Extra byte written is LZW Minimum Code Size, i dont know what it is and i dont care
                dest.write_all(&read_bytes!(src, color_table_size as usize + 1)[..])?;
                write_sub_blocks(src, dest)?;
            }
            // EOF
            0x3B => {
                dest.write_all(&[0x21, 0xFF, 0x0B])?;
                dest.write_all(IDENTIFIER)?;

                let mut tags: Vec<String> = tags.iter().cloned().collect();
                tags.sort_unstable();
                let mut tag_bytes = Vec::new();
                for tag in &mut tags {
                    tag_bytes.push(tag.len() as u8);
                    tag_bytes.append(&mut tag.as_bytes().to_vec());
                }
                tag_bytes.push(0);
                dest.write_all(&tag_bytes[..])?;

                dest.write_all(&[identifier])?;
                return Ok(());
            }
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tagset;

    #[test]
    fn normal() {
        assert_read!("normal.gif", tagset! {});
        assert_write!("normal.gif", tagset! { "foo", "bar" }, "tagged.gif");
    }

    #[test]
    fn no_tags() {
        assert_read!("no_tags.gif", tagset! {});
        assert_write!("no_tags.gif", tagset! { "foo", "bar" }, "tagged.gif");
    }

    #[test]
    fn tagged() {
        assert_read!("tagged.gif", tagset! { "foo", "bar" });
        assert_write!("tagged.gif", tagset! {}, "no_tags.gif");
    }
}
