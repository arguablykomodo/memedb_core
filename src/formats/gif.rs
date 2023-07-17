//! # Graphics Interchange Format
//!
//! GIF files are organized as a sequence of descriptors, extensions, and image data:
//!
//! - A Logical Screen Descriptor must be at the beginning of the file, it has a fixed sized and
//! may be followed by an optional color table.
//! - Extensions are identified by a `0x21` byte, followed by a label byte and a series of
//! sub-blocks.
//! - Image Descriptors start with a `0x2C` byte have a fixed size and are followed by an optional
//! color table and a series of sub-blocks.
//! - Sub-blocks indicate their size in a single byte, followed by their data. A sequence of
//! sub-blocks ends when a sub-block of size 0 is found.
//! - The file ends when a trailer block is found, indicated by a single `0x3B` byte.
//!
//! GIF files start with a fixed-length header (`GIF87a` or `GIF89a`) marking which version of the
//! spec is used. This library only handles the `GIF89a` spec.
//!
//! MemeDB stores its tags in an Application Extension with the label `MEMETAGS1.0`.
//!
//! ## Related Links
//!
//! - [Wikipedia article for GIF](https://en.wikipedia.org/wiki/GIF)
//! - [GIF Specification](https://www.w3.org/Graphics/GIF/spec-gif89a.txt)
//! - [Matthew Flickinger's "What's In A GIF"](https://www.matthewflickinger.com/lab/whatsinagif/)

pub(crate) const MAGIC: &[u8] = b"GIF89a";
pub(crate) const OFFSET: usize = 0;

use crate::{
    utils::{passthrough, read_byte, read_heap, skip},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const IDENTIFIER: &[u8; 11] = b"MEMETAGS1.0";

fn color_table_size(byte: u8) -> u16 {
    3 * 2u16.pow((byte & 0b00000111) as u32 + 1)
}

fn passthrough_blocks(src: &mut impl Read, dest: &mut impl Write) -> Result<(), std::io::Error> {
    let mut n = read_byte(src)?;
    dest.write_all(&[n])?;
    loop {
        if n == 0 {
            return Ok(());
        } else {
            let buf = read_heap(src, n as usize + 1)?;
            n = *buf.last().unwrap();
            dest.write_all(&buf)?;
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    skip(src, MAGIC.len() as i64 + 4)?;
    let packed = read_byte(src)?;
    skip(src, 2)?;
    if packed >> 7 == 1 {
        skip(src, color_table_size(packed) as i64)?;
    }
    loop {
        match read_byte(src)? {
            0x21 => {
                let label = read_byte(src)?;
                if label == 0xFF {
                    let size = read_byte(src)?;
                    let identifier = read_heap(src, size as usize)?;
                    if identifier == IDENTIFIER {
                        let mut tags = TagSet::new();
                        loop {
                            let tag_length = read_byte(src)?;
                            if tag_length == 0 {
                                return Ok(tags);
                            } else {
                                let tag_bytes = read_heap(src, tag_length as usize)?;
                                tags.insert(String::from_utf8(tag_bytes)?);
                            }
                        }
                    }
                }
                passthrough_blocks(src, &mut std::io::sink())?;
            }
            0x2C => {
                skip(src, 8)?;
                let packed = read_byte(src)?;
                if packed >> 7 == 1 {
                    skip(src, color_table_size(packed) as i64)?;
                }
                skip(src, 1)?;
                passthrough_blocks(src, &mut std::io::sink())?;
            }
            0x3B => return Ok(TagSet::new()),
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<(), Error> {
    passthrough(src, dest, MAGIC.len() as u64 + 4)?;
    let packed = read_byte(src)?;
    dest.write_all(&[packed])?;
    passthrough(src, dest, 2)?;
    if packed >> 7 == 1 {
        passthrough(src, dest, color_table_size(packed) as u64)?;
    }
    dest.write_all(&[0x21, 0xFF, 0x0B])?;
    dest.write_all(IDENTIFIER)?;
    let mut tags: Vec<String> = tags.iter().cloned().collect();
    tags.sort_unstable();
    for tag in &mut tags {
        dest.write_all(&[tag.len() as u8])?;
        dest.write_all(tag.as_bytes())?;
    }
    dest.write_all(&[0])?;
    loop {
        let byte = read_byte(src)?;
        match byte {
            0x21 => {
                let label = read_byte(src)?;
                if label == 0xFF {
                    let size = read_byte(src)?;
                    let identifier = read_heap(src, size as usize)?;
                    if identifier == IDENTIFIER {
                        passthrough_blocks(src, &mut std::io::sink())?;
                    } else {
                        dest.write_all(&[byte, label, size])?;
                        dest.write_all(&identifier)?;
                        passthrough_blocks(src, dest)?;
                    }
                } else {
                    dest.write_all(&[byte, label])?;
                    passthrough_blocks(src, dest)?;
                }
            }
            0x2C => {
                dest.write_all(&[byte])?;
                passthrough(src, dest, 8)?;
                let packed = read_byte(src)?;
                dest.write_all(&[packed])?;
                if packed >> 7 == 1 {
                    passthrough(src, dest, color_table_size(packed) as u64)?;
                }
                passthrough(src, dest, 1)?;
                passthrough_blocks(src, dest)?;
            }
            0x3B => {
                dest.write_all(&[byte])?;
                return Ok(());
            }
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

crate::utils::standard_tests!("gif");
