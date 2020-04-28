use crate::{
    error::{Error, Result},
    TagSet,
};
use crc::Hasher32;
use std::io::{Read, Seek, SeekFrom, Write};

pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

const MEME_CHUNK: &[u8; 4] = b"meMe";

// Utility macro for easily getting bytes from a stream
macro_rules! read_bytes {
    // Use the stack if the length is known at compile-time
    ($src:expr, $n:literal) => {{
        let mut bytes = [0; $n];
        $src.read_exact(&mut bytes)?;
        bytes
    }};
    // Use the heap otherwise
    ($src:expr, $n:expr) => {{
        let mut bytes = vec![0; $n];
        $src.read_exact(&mut bytes)?;
        bytes
    }};
}

// Encodes a 4 bit big endian number.
fn encode_big_endian(n: u32) -> [u8; 4] {
    [(n >> 24 & 0xFF) as u8, (n >> 16 & 0xFF) as u8, (n >> 8 & 0xFF) as u8, (n & 0xFF) as u8]
}

// Decodes a 4 bit big endian number.
fn decode_big_endian(src: &mut impl Read) -> Result<u32> {
    Ok(read_bytes!(src, 4).iter().fold(0, |acc, n| (acc << 8) + *n as u32))
}

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
        let chunk_length = decode_big_endian(src)?;
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            b"IEND" => return Ok(tags),
            MEME_CHUNK => {
                let bytes = read_bytes!(src, chunk_length as usize);
                let checksum = decode_big_endian(src)?;
                let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
                digest.write(&chunk_type);
                digest.write(&bytes);
                if checksum != digest.sum32() {
                    return Err(Error::PngChecksum);
                }
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
            _ => {
                // Skip unknown chunks
                src.seek(SeekFrom::Current(chunk_length as i64 + 4))?;
            }
        }
    }
}

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    loop {
        let chunk_length = decode_big_endian(src)?;
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            b"IEND" => {
                let mut tags: Vec<_> = tags.into_iter().collect();
                tags.sort_unstable();
                let tags: Vec<_> =
                    tags.into_iter().map(|t| (t + ";").into_bytes()).flatten().collect();

                if tags.len() as u64 >= std::u32::MAX as u64 {
                    return Err(Error::PngOverflow);
                }

                let checksum = {
                    let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
                    digest.write(MEME_CHUNK);
                    digest.write(&tags);
                    digest.sum32()
                };

                let mut buffer = Vec::new();
                buffer.extend(&encode_big_endian(tags.len() as u32));
                buffer.extend(MEME_CHUNK);
                buffer.extend(tags);
                buffer.extend(&encode_big_endian(checksum));

                dest.write_all(&buffer)?;

                // Write rest of the file
                dest.write_all(&encode_big_endian(chunk_length))?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;

                return Ok(());
            }
            MEME_CHUNK => {
                // Skip existing meme chunks
                src.seek(SeekFrom::Current(chunk_length as i64 + 4))?;
            }
            _ => {
                // Write unrelated chunks unchanged
                dest.write_all(&encode_big_endian(chunk_length))?;
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

    macro_rules! assert_read {
        ($file:literal, $tags:expr) => {
            let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $file));
            let mut cursor = std::io::Cursor::new(&bytes[..]);
            cursor.set_position(SIGNATURE.len() as u64);
            assert_eq!(read_tags(&mut cursor).unwrap(), $tags);
        };
    }

    #[test]
    fn normal() {
        assert_read!("normal.png", tagset! {});
    }

    #[test]
    fn no_tags() {
        assert_read!("no_tags.png", tagset! {});
    }

    #[test]
    fn tagged() {
        assert_read!("tagged.png", tagset! { "foo", "bar" });
    }

    #[test]
    fn multiple_chunks() {
        assert_read!("multiple_chunks.png", tagset! { "foo", "bar" });
    }
}
