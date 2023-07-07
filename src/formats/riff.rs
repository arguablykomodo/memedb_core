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
    utils::{read_heap, read_stack, skip},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const TAG_CHUNK: &[u8; 4] = b"meme";

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    skip(src, MAGIC.len() as i64)?;
    let mut file_length = u32::from_le_bytes(read_stack::<4>(src)?);
    skip(src, 4)?;
    file_length = file_length.checked_sub(4).ok_or(Error::InvalidRiffLength)?;
    while file_length > 0 {
        let name = read_stack::<4>(src)?;
        let length = u32::from_le_bytes(read_stack::<4>(src)?);
        if &name == TAG_CHUNK {
            let mut bytes = read_heap(src, length as usize)?;
            let mut tags = TagSet::new();
            while !bytes.is_empty() {
                let size = bytes.remove(0) as usize;
                let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                tags.insert(String::from_utf8(bytes)?);
            }
            return Ok(tags);
        }
        skip(src, length as i64)?;
        if (length & 1) == 1 {
            skip(src, 1)?;
            file_length = file_length.checked_sub(1).ok_or(Error::InvalidRiffLength)?;
        }
        // Name + length + payload
        file_length = file_length.checked_sub(4 + 4).ok_or(Error::InvalidRiffLength)?;
        file_length = file_length.checked_sub(length).ok_or(Error::InvalidRiffLength)?;
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
    skip(src, MAGIC.len() as i64)?;
    dest.write_all(MAGIC)?;

    let mut file_length = u32::from_le_bytes(read_stack::<4>(src)?);

    // Because we need to write the length of the file at the beggining, we need to store
    // everything in a buffer before writing. Those four 0x0 bytes are placeholders for the final
    // length
    let mut buffer = vec![0, 0, 0, 0];

    buffer.extend_from_slice(&read_stack::<4>(src)?);
    file_length = file_length.checked_sub(4).ok_or(Error::InvalidRiffLength)?;

    while file_length > 0 {
        let name = read_stack::<4>(src)?;
        let chunk_length_bytes = read_stack::<4>(src)?;
        let chunk_length = u32::from_le_bytes(chunk_length_bytes);
        if &name == TAG_CHUNK {
            skip(src, chunk_length as i64)?;
            if (chunk_length & 1) == 1 {
                skip(src, 1)?;
                file_length = file_length.checked_sub(1).ok_or(Error::InvalidRiffLength)?;
            }
        } else {
            buffer.extend_from_slice(&name);
            buffer.extend_from_slice(&chunk_length_bytes);
            buffer.extend_from_slice(&read_heap(src, chunk_length as usize)?);
            if (chunk_length & 1) == 1 {
                buffer.push(0);
                file_length = file_length.checked_sub(1).ok_or(Error::InvalidRiffLength)?;
            }
        }
        // Name + length + payload
        file_length = file_length.checked_sub(4 + 4).ok_or(Error::InvalidRiffLength)?;
        file_length = file_length.checked_sub(chunk_length).ok_or(Error::InvalidRiffLength)?;
    }

    // We have to store the tags at the end because webp wants the chunks to be in a specific order
    // So yeah, thanks webp
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tag_bytes = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.extend(tag.into_bytes());
        acc
    });

    if tag_bytes.len() as u64 >= std::u32::MAX as u64 {
        return Err(Error::ChunkSizeOverflow);
    }

    let tags_length = tag_bytes.len() as u32;
    buffer.extend_from_slice(TAG_CHUNK);
    buffer.extend(tags_length.to_le_bytes().iter());
    buffer.extend(tag_bytes.into_iter());
    if (tags_length & 1) == 1 {
        buffer.push(0);
    }

    // We subtract 4 here because the bytes storing the size are not counted
    let final_length = (buffer.len() as u32 - 4).to_le_bytes();
    buffer[0] = final_length[0]; // THIS
    buffer[1] = final_length[1]; // IS
    buffer[2] = final_length[2]; // VERY
    buffer[3] = final_length[3]; // DUMB

    dest.write_all(&buffer)?;

    Ok(())
}

crate::utils::standard_tests!("webp");
