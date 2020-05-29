use crate::{
    error::{Error, Result},
    TagSet,
};
use crc::Hasher32;
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

const TAG_CHUNK: &[u8; 4] = b"meMe";
const END_CHUNK: &[u8; 4] = b"IEND";

// PNG data is stored in chunks:
// Each chunk starts with a 4 byte big endian number describing the length of the data within.
// After that there's a 4 byte ASCII identifier for the chunk type (meMe in our case).
// Then comes the data, which is as long as the length described.
// We store tags as utf8, with each tag ending with a semicolon.
// And at the end there is a CRC-32 checksum of the chunk type and data.
// An IEND chunk marks the end of the file
// Related links:
// http://www.libpng.org/pub/png/apps/pngcheck.html
// https://en.wikipedia.org/wiki/Portable_Network_Graphics

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    let mut tags = crate::TagSet::new();
    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            END_CHUNK => return Ok(tags),
            TAG_CHUNK => {
                let bytes = read_bytes!(src, chunk_length as usize);

                // Verify checksum
                let checksum = u32::from_be_bytes(read_bytes!(src, 4));
                let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
                digest.write(&chunk_type);
                digest.write(&bytes);
                if checksum != digest.sum32() {
                    return Err(Error::PngChecksum);
                }

                // Collect tags, split at semicolons
                let mut tag = String::new();
                for byte in bytes {
                    match byte {
                        b';' => {
                            tags.insert(std::mem::replace(&mut tag, String::new()));
                        }
                        _ => tag.push(byte as char),
                    }
                }
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
    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            END_CHUNK => {
                // Encode tags
                let mut tags: Vec<_> = tags.into_iter().collect();
                tags.sort_unstable();
                let tags: Vec<_> =
                    tags.into_iter().map(|t| (t + ";").into_bytes()).flatten().collect();

                // If this error is returned, someone has *way* too many tags
                if tags.len() as u64 >= std::u32::MAX as u64 {
                    return Err(Error::PngOverflow);
                }

                // Compute checksum
                let checksum = {
                    let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
                    digest.write(TAG_CHUNK);
                    digest.write(&tags);
                    digest.sum32()
                };

                // Write it all
                let mut buffer = Vec::new();
                buffer.extend(&(tags.len() as u32).to_be_bytes());
                buffer.extend(TAG_CHUNK);
                buffer.extend(tags);
                buffer.extend(&checksum.to_be_bytes());
                dest.write_all(&buffer)?;

                // Write rest of the file
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;

                return Ok(());
            }
            // Skip old tags
            TAG_CHUNK => {
                skip_bytes!(src, chunk_length as i64 + 4)?;
            }
            // Leave unrelated chunks unchanged
            _ => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tagset;

    #[test]
    fn normal() {
        assert_read!("normal.png", tagset! {});
        assert_write!("normal.png", tagset! { "foo", "bar" }, "tagged.png");
    }

    #[test]
    fn no_tags() {
        assert_read!("no_tags.png", tagset! {});
        assert_write!("no_tags.png", tagset! { "foo", "bar" }, "tagged.png");
    }

    #[test]
    fn tagged() {
        assert_read!("tagged.png", tagset! { "foo", "bar" });
        assert_write!("tagged.png", tagset! {}, "no_tags.png");
    }

    #[test]
    fn multiple_chunks() {
        assert_read!("multiple_chunks.png", tagset! { "foo", "bar" });
        assert_write!("tagged.png", tagset! { "foo", "bar" }, "tagged.png");
    }

    #[test]
    fn when_you() {
        assert_read!("when_you.png", tagset! {});
    }
}
