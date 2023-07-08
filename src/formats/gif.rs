//! # Graphics Interchange Format
//!
//! GIF files are organized as a sequence of descriptors, extensions, and data, with varying types
//! and encodings:
//!
//! - A Logical Screen Descriptor must be at the beginning of the file, it has a fixed sized and
//! may be followed by an optional color table.
//! - Extensions are identified by a `0x21` byte, followed by a label byte and a length byte.
//! - Image Descriptors have a fixed size and are followed by an optional color table and image
//! data.
//! - Variable sized data (like the one found after extensions and in image data) is stored in
//! sub-blocks, which indicate their sized in a single byte, followed by the data. A sequence of
//! sub-blocks ends when a sub-block of size 0 is found.
//! - The file ends when a trailer byte is found (`0x3B`)
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
    utils::{passthrough, read_byte, read_heap, read_stack, skip},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const IDENTIFIER: &[u8; 11] = b"MEMETAGS1.0";

fn skip_sub_blocks(src: &mut (impl Read + Seek)) -> Result<(), Error> {
    loop {
        let sub_block_length = read_byte(src)?;
        if sub_block_length == 0 {
            return Ok(());
        } else {
            skip(src, sub_block_length as i64)?;
        }
    }
}

fn write_sub_blocks(src: &mut impl Read, dest: &mut impl Write) -> Result<(), Error> {
    loop {
        let sub_block_length = read_byte(src)?;
        dest.write_all(&[sub_block_length])?;
        if sub_block_length == 0 {
            return Ok(());
        } else {
            passthrough(src, dest, sub_block_length as u64)?;
        }
    }
}

#[allow(clippy::unreadable_literal)]
fn get_color_table_size(byte: u8) -> u16 {
    let has_global_color_table = byte & 0b10000000;
    if has_global_color_table >> 7 == 1 {
        let packed_size = byte & 0b00000111;
        3 * 2u16.pow(packed_size as u32 + 1)
    } else {
        0
    }
}

enum Section {
    Tags(u8, u8, u8, [u8; 11]),
    Application(u8, u8, u8, [u8; 11]),
    Comment(u8, u8),
    GraphicsControl(u8, u8),
    Plaintext(u8, u8),
    ImageDescriptor(u8),
    Eof(u8),
}
use Section::*;

fn get_section(src: &mut impl Read) -> Result<Section, Error> {
    let identifier = read_byte(src)?;
    Ok(match identifier {
        // Extension
        0x21 => {
            let extension_type = read_byte(src)?;
            match extension_type {
                // Application Extension
                0xFF => {
                    let block_size = read_byte(src)?;
                    if block_size != 11 {
                        return Err(Error::InvalidSource("wrong block size"));
                    }
                    let application_identifier = read_stack::<11>(src)?;
                    if &application_identifier == IDENTIFIER {
                        Tags(identifier, extension_type, block_size, application_identifier)
                    } else {
                        Application(identifier, extension_type, block_size, application_identifier)
                    }
                }
                0xFE => Comment(identifier, extension_type),
                0xF9 => GraphicsControl(identifier, extension_type),
                0x01 => Plaintext(identifier, extension_type),
                _ => return Err(Error::InvalidSource("unknown extension type")),
            }
        }
        0x2C => ImageDescriptor(identifier),
        0x3B => Eof(identifier),
        _ => return Err(Error::InvalidSource("malformed gif data")),
    })
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    skip(src, MAGIC.len() as i64)?;
    let logical_screen_descriptor = read_stack::<7>(src)?;
    let color_table_size = get_color_table_size(logical_screen_descriptor[4]);
    skip(src, color_table_size as i64)?;

    loop {
        match get_section(src)? {
            Tags(_, _, _, _) => {
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
            Application(_, _, _, _) | Comment(_, _) => skip_sub_blocks(src)?,
            GraphicsControl(_, _) | Plaintext(_, _) => {
                let block_size = read_byte(src)?;
                skip(src, block_size as i64)?;
                skip_sub_blocks(src)?;
            }
            ImageDescriptor(_) => {
                let data = read_stack::<9>(src)?;
                let color_table_size = get_color_table_size(data[8]);
                // Extra byte skipped is LZW Minimum Code Size, i dont know what it is and i dont care
                skip(src, color_table_size as i64 + 1)?;
                skip_sub_blocks(src)?;
            }
            Eof(_) => return Ok(TagSet::new()),
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
    skip(src, MAGIC.len() as i64)?;
    dest.write_all(MAGIC)?;

    let logical_screen_descriptor = read_stack::<7>(src)?;
    dest.write_all(&logical_screen_descriptor)?;
    let color_table_size = get_color_table_size(logical_screen_descriptor[4]);
    passthrough(src, dest, color_table_size as u64)?;

    // Write tags
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

    loop {
        match get_section(src)? {
            Tags(_, _, _, _) => skip_sub_blocks(src)?,
            Application(identifier, extension_type, block_size, application_identifier) => {
                dest.write_all(&[identifier, extension_type, block_size])?;
                dest.write_all(&application_identifier[..])?;
                write_sub_blocks(src, dest)?;
            }
            Comment(identifier, extension_type) => {
                dest.write_all(&[identifier, extension_type])?;
                write_sub_blocks(src, dest)?;
            }
            GraphicsControl(identifier, extension_type) | Plaintext(identifier, extension_type) => {
                dest.write_all(&[identifier, extension_type])?;
                let block_size = read_byte(src)?;
                dest.write_all(&[block_size])?;
                passthrough(src, dest, block_size as u64)?;
                write_sub_blocks(src, dest)?;
            }
            ImageDescriptor(identifier) => {
                dest.write_all(&[identifier])?;
                let data = read_stack::<9>(src)?;
                dest.write_all(&data)?;
                let color_table_size = get_color_table_size(data[8]);
                // Extra byte written is LZW Minimum Code Size, i dont know what it is and i dont care
                passthrough(src, dest, color_table_size as u64 + 1)?;
                write_sub_blocks(src, dest)?;
            }
            Eof(identifier) => {
                dest.write_all(&[identifier])?;
                return Ok(());
            }
        }
    }
}

crate::utils::standard_tests!("gif");
