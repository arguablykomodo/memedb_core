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
    utils::{passthrough, read_heap, read_stack, skip, read_byte, or_eof},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const TAG_CHUNK: &[u8; 4] = b"meMe";
const END_CHUNK: &[u8; 4] = b"IEND";

const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    skip(src, MAGIC.len() as i64)?;
    loop {
        let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
        let chunk_type = read_stack::<4>(src)?;
        match &chunk_type {
            END_CHUNK => return Ok(TagSet::new()),
            TAG_CHUNK => {
                let mut digest = CRC.digest();
                digest.update(&chunk_type);

                let mut tags = TagSet::new();
                let mut tag_src = src.take(chunk_length as u64);
                while let Some(n) = or_eof(read_byte(&mut tag_src))? {
                    let tag = read_heap(&mut tag_src, n as usize)?;
                    digest.update(&[n]);
                    digest.update(&tag);
                    tags.insert(String::from_utf8(tag)?);
                }

                let checksum = u32::from_be_bytes(read_stack::<4>(src)?);
                if checksum != digest.finalize() {
                    return Err(Error::InvalidSource("wrong checksum"));
                }

                return Ok(tags);
            }
            // We dont care about these, skip!
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
    tags: TagSet,
) -> Result<(), Error> {
    skip(src, MAGIC.len() as i64)?;
    dest.write_all(MAGIC)?;

    // The first chunk should always be IHDR, according to the spec, so we are going to read it manually
    let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
    let chunk_type = read_stack::<4>(src)?;
    dest.write_all(&chunk_length.to_be_bytes())?;
    dest.write_all(&chunk_type)?;
    passthrough(src, dest, chunk_length as u64 + 4)?;

    // Encode tags
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tags = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.append(&mut tag.into_bytes());
        acc
    });

    // If this error is returned, someone has *way* too many tags
    if tags.len() as u64 >= std::u32::MAX as u64 {
        return Err(Error::ChunkSizeOverflow);
    }

    // Compute checksum
    let checksum = {
        let mut digest = CRC.digest();
        digest.update(TAG_CHUNK);
        digest.update(&tags);
        digest.finalize()
    };

    // Write tag chunk
    let mut buffer = Vec::new();
    buffer.extend((tags.len() as u32).to_be_bytes());
    buffer.extend(TAG_CHUNK);
    buffer.extend(tags);
    buffer.extend(checksum.to_be_bytes());
    dest.write_all(&buffer)?;

    loop {
        let chunk_length = u32::from_be_bytes(read_stack::<4>(src)?);
        let chunk_type = read_stack::<4>(src)?;
        match &chunk_type {
            // Skip old tags
            TAG_CHUNK => {
                skip(src, chunk_length as i64 + 4)?;
            }
            // Write rest of the file
            END_CHUNK => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                passthrough(src, dest, chunk_length as u64 + 4)?;
                return Ok(());
            }
            // Leave unrelated chunks unchanged
            _ => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                passthrough(src, dest, chunk_length as u64 + 4)?;
            }
        }
    }
}

crate::utils::standard_tests!("png");
