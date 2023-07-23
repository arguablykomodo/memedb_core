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

use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{
    utils::{
        decode_tags, decode_tags_async, encode_tags, encode_tags_async, passthrough,
        passthrough_async, read_byte, read_byte_async, read_heap, read_heap_async, skip,
        skip_async,
    },
    Error,
};
use std::io::{Read, Seek, Write};

const IDENTIFIER: &[u8; 11] = b"MEMETAGS1.0";

fn color_table_size(byte: u8) -> u16 {
    3 * 2u16.pow((byte & 0b00000111) as u32 + 1)
}

async fn passthrough_blocks_async(
    src: &mut (impl AsyncReadExt + Unpin),
    dest: &mut (impl AsyncWriteExt + Unpin),
) -> Result<(), std::io::Error> {
    let mut n = read_byte_async(src).await?;
    dest.write_all(&[n]).await?;
    loop {
        if n == 0 {
            return Ok(());
        }
        let buf = read_heap_async(src, n as usize + 1).await?;
        n = *buf.last().unwrap();
        dest.write_all(&buf).await?;
    }
}

fn passthrough_blocks(src: &mut impl Read, dest: &mut impl Write) -> Result<(), std::io::Error> {
    let mut n = read_byte(src)?;
    dest.write_all(&[n])?;
    loop {
        if n == 0 {
            return Ok(());
        }
        let buf = read_heap(src, n as usize + 1)?;
        n = *buf.last().unwrap();
        dest.write_all(&buf)?;
    }
}

/// Given a `src`, return the tags contained inside.
pub async fn read_tags_async(
    src: &mut (impl AsyncReadExt + AsyncSeekExt + Unpin),
) -> Result<Vec<String>, Error> {
    skip_async(src, MAGIC.len() as i64 + 4).await?;
    let packed = read_byte_async(src).await?;
    skip_async(src, 2).await?;
    if packed >> 7 == 1 {
        skip_async(src, color_table_size(packed) as i64).await?;
    }
    loop {
        match read_byte_async(src).await? {
            0x21 => {
                let label = read_byte_async(src).await?;
                if label == 0xFF {
                    let size = read_byte_async(src).await?;
                    let identifier = read_heap_async(src, size as usize).await?;
                    if identifier == IDENTIFIER {
                        let mut tags_bytes = Vec::new();
                        let mut n = read_byte_async(src).await?;
                        loop {
                            if n == 0 {
                                break;
                            }
                            let buf = read_heap_async(src, n as usize + 1).await?;
                            tags_bytes.extend(&buf[..n as usize]);
                            n = *buf.last().unwrap();
                        }
                        return decode_tags_async(&mut tags_bytes.as_slice()).await;
                    }
                }
                passthrough_blocks_async(src, &mut futures::io::sink()).await?;
            }
            0x2C => {
                skip_async(src, 8).await?;
                let packed = read_byte_async(src).await?;
                if packed >> 7 == 1 {
                    skip_async(src, color_table_size(packed) as i64).await?;
                }
                skip_async(src, 1).await?;
                passthrough_blocks_async(src, &mut futures::io::sink()).await?;
            }
            0x3B => return Ok(Vec::new()),
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Vec<String>, Error> {
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
                        let mut tags_bytes = Vec::new();
                        let mut n = read_byte(src)?;
                        loop {
                            if n == 0 {
                                break;
                            }
                            let buf = read_heap(src, n as usize + 1)?;
                            tags_bytes.extend(&buf[..n as usize]);
                            n = *buf.last().unwrap();
                        }
                        return decode_tags(&mut tags_bytes.as_slice());
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
            0x3B => return Ok(Vec::new()),
            byte => return Err(Error::GifUnknownBlock(byte)),
        }
    }
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub async fn write_tags_async(
    src: &mut (impl AsyncReadExt + AsyncSeekExt + Unpin),
    dest: &mut (impl AsyncWriteExt + Unpin),
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    passthrough_async(src, dest, MAGIC.len() as u64 + 4).await?;
    let packed = read_byte_async(src).await?;
    dest.write_all(&[packed]).await?;
    passthrough_async(src, dest, 2).await?;
    if packed >> 7 == 1 {
        passthrough_async(src, dest, color_table_size(packed) as u64).await?;
    }
    dest.write_all(&[0x21, 0xFF, IDENTIFIER.len() as u8]).await?;
    dest.write_all(IDENTIFIER).await?;
    let mut tag_bytes = Vec::new();
    encode_tags_async(tags, std::pin::pin!(&mut tag_bytes)).await?;
    let mut tag_slice = tag_bytes.as_slice();
    while !tag_slice.is_empty() {
        let sub_block_size = tag_slice.len().min(0xFF);
        dest.write_all(&[sub_block_size as u8]).await?;
        dest.write_all(&tag_slice[0..sub_block_size]).await?;
        tag_slice = &tag_slice[sub_block_size..];
    }
    dest.write_all(&[0]).await?;
    loop {
        let byte = read_byte_async(src).await?;
        match byte {
            0x21 => {
                let label = read_byte_async(src).await?;
                if label == 0xFF {
                    let size = read_byte_async(src).await?;
                    let identifier = read_heap_async(src, size as usize).await?;
                    if identifier == IDENTIFIER {
                        passthrough_blocks_async(src, &mut futures::io::sink()).await?;
                    } else {
                        dest.write_all(&[byte, label, size]).await?;
                        dest.write_all(&identifier).await?;
                        passthrough_blocks_async(src, dest).await?;
                    }
                } else {
                    dest.write_all(&[byte, label]).await?;
                    passthrough_blocks_async(src, dest).await?;
                }
            }
            0x2C => {
                dest.write_all(&[byte]).await?;
                passthrough_async(src, dest, 8).await?;
                let packed = read_byte_async(src).await?;
                dest.write_all(&[packed]).await?;
                if packed >> 7 == 1 {
                    passthrough_async(src, dest, color_table_size(packed) as u64).await?;
                }
                passthrough_async(src, dest, 1).await?;
                passthrough_blocks_async(src, dest).await?;
            }
            0x3B => {
                dest.write_all(&[byte]).await?;
                return Ok(());
            }
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
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    passthrough(src, dest, MAGIC.len() as u64 + 4)?;
    let packed = read_byte(src)?;
    dest.write_all(&[packed])?;
    passthrough(src, dest, 2)?;
    if packed >> 7 == 1 {
        passthrough(src, dest, color_table_size(packed) as u64)?;
    }
    dest.write_all(&[0x21, 0xFF, IDENTIFIER.len() as u8])?;
    dest.write_all(IDENTIFIER)?;
    let mut tag_bytes = Vec::new();
    encode_tags(tags, &mut tag_bytes)?;
    let mut tag_slice = tag_bytes.as_slice();
    while !tag_slice.is_empty() {
        let sub_block_size = tag_slice.len().min(0xFF);
        dest.write_all(&[sub_block_size as u8])?;
        dest.write_all(&tag_slice[0..sub_block_size])?;
        tag_slice = &tag_slice[sub_block_size..];
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
