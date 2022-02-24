// PNG data is stored in chunks: Each chunk starts with a 4 byte big endian number describing the
// length of the data within. After that there's a 4 byte ASCII identifier for the chunk type.
// Then comes the data, which is as long as the length described. At the end there is a CRC-32
// checksum of the chunk type and data. An IEND chunk marks the end of the file.
//
// we store tags in the `meMe` chunk. The tags are encoded by a byte storing the length of the tag,
// followed by the UTF-8 encoded bytes.
//
// Related links:
// http://www.libpng.org/pub/png/apps/pngcheck.html
// https://en.wikipedia.org/wiki/Portable_Network_Graphics

use crate::{
    error::{Error, Result},
    TagSet,
};
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = b"\x89PNG\x0D\x0A\x1A\x0A";

const TAG_CHUNK: &[u8; 4] = b"meMe";
const END_CHUNK: &[u8; 4] = b"IEND";

const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            END_CHUNK => return Ok(crate::TagSet::new()),
            TAG_CHUNK => {
                let mut bytes = read_bytes!(src, chunk_length as u64);

                // Verify checksum
                let checksum = u32::from_be_bytes(read_bytes!(src, 4));
                let mut digest = CRC.digest();
                digest.update(&chunk_type);
                digest.update(&bytes);
                if checksum != digest.finalize() {
                    return Err(Error::PngChecksum);
                }

                // Collect tags
                let mut tags = TagSet::new();
                while !bytes.is_empty() {
                    let size = bytes.remove(0) as usize;
                    let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                    tags.insert(String::from_utf8(bytes)?);
                }
                return Ok(tags);
            }
            // We dont care about these, skip!
            _ => {
                skip_bytes!(src, chunk_length as i64 + 4)?;
            }
        }
    }
}

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    dest.write_all(SIGNATURE)?;

    // The first chunk should always be IHDR, according to the spec, so we are going to read it manually
    let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
    let chunk_type = read_bytes!(src, 4);
    dest.write_all(&chunk_length.to_be_bytes())?;
    dest.write_all(&chunk_type)?;
    dest.write_all(&read_bytes!(src, chunk_length as u64 + 4))?;

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
    buffer.extend(&(tags.len() as u32).to_be_bytes());
    buffer.extend(TAG_CHUNK);
    buffer.extend(tags);
    buffer.extend(&checksum.to_be_bytes());
    dest.write_all(&buffer)?;

    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            // Skip old tags
            TAG_CHUNK => {
                skip_bytes!(src, chunk_length as i64 + 4)?;
            }
            // Write rest of the file
            END_CHUNK => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as u64 + 4))?;
                return Ok(());
            }
            // Leave unrelated chunks unchanged
            _ => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as u64 + 4))?;
            }
        }
    }
}

#[cfg(test)]
make_tests! {"png"}
