//! # Portable Network Graphics
//!
//! PNG data is organized in chunks. Each chunk is structured as follows:
//!
//! - 4 byte big endian number describing the length of the data within.
//! - 4 byte ASCII identifier for the chunk type.
//! - The chunk data itself, which is as long as described in the length field.
//! - 4 byte CRC-32 checksum of the chunk type and data.
//!
//! A PNG file starts with a magic number to identify itself, followed by a series of chunks, the
//! first of which must be `IHDR`, and the last of which must be `IEND`.
//!
//! MemeDB stores its tags in a `meMe` chunk.
//!
//! ## Relevant Links
//!
//! - [Wikipedia article for PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics)
//! - [The PNG specification](https://www.w3.org/TR/2003/REC-PNG-20031110/)
//! - [`pngcheck`, a program to analyze PNG files](http://www.libpng.org/pub/png/apps/pngcheck.html)

pub(crate) const MAGIC: &[u8] = b"\x89PNG\x0D\x0A\x1A\x0A";
pub(crate) const OFFSET: usize = 0;

use crate::{
    utils::{encode_tags, passthrough, read_byte, read_stack, skip, read_heap},
    Error,
};
use std::io::{Read, Seek, Write};

const TAG_CHUNK: &[u8; 4] = b"meMe";
const END_CHUNK: &[u8; 4] = b"IEND";

const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Vec<String>, Error> {
    skip(src, MAGIC.len() as i64)?;
    loop {
        let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
        let chunk_type = read_stack::<4>(src)?;
        match &chunk_type {
            END_CHUNK => return Ok(Vec::new()),
            TAG_CHUNK => {
                let mut digest = CRC.digest();
                digest.update(&chunk_type);
                let mut tags = Vec::new();
                let mut tag_bytes = Vec::new();
                loop {
                    let byte = read_byte(src)?;
                    digest.update(&[byte]);
                    match byte {
                        0b00000000 => break,
                        0b00000001..=0b01111111 => {
                            let bytes = read_heap(src, byte as usize)?;
                            digest.update(&bytes);
                            tag_bytes.extend(bytes);
                            continue;
                        }
                        0b10000000..=0b11111111 => {
                            let bytes = read_heap(src, (byte & 0b01111111) as usize)?;
                            digest.update(&bytes);
                            tag_bytes.extend(bytes);
                            tags.push(String::from_utf8(tag_bytes)?);
                            tag_bytes = Vec::new();
                        }
                    }
                }
                let checksum = u32::from_be_bytes(read_stack::<4>(src)?);
                if checksum != digest.finalize() {
                    return Err(Error::PngChecksum);
                }
                return Ok(tags);
            }
            _ => {
                skip(src, chunk_length as i64 + 4)?;
            }
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
    passthrough(src, dest, MAGIC.len() as u64)?;
    // Passthrough first IHDR chunk
    let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
    let chunk_type = read_stack::<4>(src)?;
    dest.write_all(&chunk_length.to_be_bytes())?;
    dest.write_all(&chunk_type)?;
    passthrough(src, dest, chunk_length as u64 + 4)?;

    let mut digest = CRC.digest();
    digest.update(TAG_CHUNK);
    let mut tags_bytes = Vec::new();
    encode_tags(tags, &mut tags_bytes)?;
    digest.update(&tags_bytes);
    dest.write_all(&(tags_bytes.len() as u32).to_be_bytes())?;
    dest.write_all(TAG_CHUNK)?;
    dest.write_all(&tags_bytes)?;
    dest.write_all(&digest.finalize().to_be_bytes())?;

    loop {
        let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
        let chunk_type = read_stack::<4>(src)?;
        match &chunk_type {
            TAG_CHUNK => {
                skip(src, chunk_length as i64 + 4)?;
            }
            END_CHUNK => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                passthrough(src, dest, chunk_length as u64 + 4)?;
                return Ok(());
            }
            _ => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                passthrough(src, dest, chunk_length as u64 + 4)?;
            }
        }
    }
}

crate::utils::standard_tests!("png");
