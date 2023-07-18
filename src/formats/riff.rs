//! # Resource Interchange File Format
//!
//! RIFF data is organized in chunks. Each chunk is structured as follows:
//!
//! - 4 byte ASCII name.
//! - 4 byte little endian length.
//! - The chunk data itself.
//! - An extra padding byte if the length is not even.
//!
//! A RIFF file is composed of a single `RIFF` meta-chunk, that contains a 4-byte ASCII name
//! describing the format of the payload (`WEBP`, `AVI `, `WAV `, etc), and then a series of
//! sub-chunks.
//!
//! MemeDB stores its tags in a `meme` chunk.
//!
//! ## Relevant Links
//!
//! - [Wikipedia article for RIFF](https://en.wikipedia.org/wiki/Resource_Interchange_File_Format)
//! - [WebP Container Specification](https://developers.google.com/speed/webp/docs/riff_container)

pub(crate) const MAGIC: &[u8] = b"RIFF";
pub(crate) const OFFSET: usize = 0;

use crate::{
    utils::{or_eof, passthrough, read_byte, read_heap, read_stack, skip},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const TAGS_ID: &[u8; 4] = b"meme";

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    let _ = read_stack::<12>(src)?; // We dont care about them, but they have to be there
    while let Some(chunk_id) = or_eof(read_stack::<4>(src))? {
        let chunk_size = u32::from_le_bytes(read_stack::<4>(src)?);
        match &chunk_id {
            TAGS_ID => {
                let mut tags = TagSet::new();
                let mut data = src.take(chunk_size as u64);
                while let Some(n) = or_eof(read_byte(&mut data))? {
                    tags.insert(String::from_utf8(read_heap(&mut data, n as usize)?)?);
                }
                return Ok(tags);
            }
            _ => {
                skip(src, chunk_size as i64)?;
                if chunk_size & 1 == 1 {
                    skip(src, 1)?;
                }
            }
        }
    }
    Ok(TagSet::new())
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<(), Error> {
    passthrough(src, dest, 4)?;
    skip(src, 4)?;
    let mut data = Vec::new();
    passthrough(src, &mut data, 4)?;
    while let Some(chunk_id) = or_eof(read_stack::<4>(src))? {
        let chunk_size_bytes = read_stack::<4>(src)?;
        let chunk_size = u32::from_le_bytes(chunk_size_bytes);
        match &chunk_id {
            TAGS_ID => {
                skip(src, chunk_size as i64)?;
                if chunk_size & 1 == 1 {
                    skip(src, 1)?;
                }
            }
            _ => {
                data.write_all(&chunk_id)?;
                data.write_all(&chunk_size_bytes)?;
                if passthrough(src, &mut data, chunk_size as u64)? != chunk_size as u64 {
                    return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
                };
                if chunk_size & 1 == 1 {
                    dest.write_all(&[0])?;
                }
            }
        }
    }
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tags = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.append(&mut tag.into_bytes());
        acc
    });
    data.write_all(TAGS_ID)?;
    data.write_all(&(tags.len() as u32).to_le_bytes())?;
    data.write_all(&tags)?;
    if tags.len() & 1 == 1 {
        data.write_all(&[0])?;
    }
    dest.write_all(&(data.len() as u32).to_le_bytes())?;
    dest.write_all(&data)?;
    Ok(())
}

crate::utils::standard_tests!("webp");
