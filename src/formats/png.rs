use crate::error::{Error, Result};
use crc::Hasher32;
use std::io;

pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

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

// Decodes a 4 bit big endian number.
fn decode_big_endian(src: &mut impl io::Read) -> Result<u32> {
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

pub fn read_tags(src: &mut (impl io::Read + io::Seek)) -> Result<crate::TagSet> {
    let mut tags = crate::TagSet::new();
    loop {
        let chunk_length = decode_big_endian(src)?;
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            b"IEND" => return Ok(tags),
            b"meMe" => {
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
                src.seek(io::SeekFrom::Current(chunk_length as i64 + 4))?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tagset;

    macro_rules! assert_tags {
        ($file:literal, $tags:expr) => {
            let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $file));
            let mut cursor = std::io::Cursor::new(&bytes[..]);
            cursor.set_position(SIGNATURE.len() as u64);
            assert_eq!(read_tags(&mut cursor).unwrap(), $tags);
        };
    }

    #[test]
    fn normal() {
        assert_tags!("normal.png", tagset! {});
    }

    #[test]
    fn no_tags() {
        assert_tags!("no_tags.png", tagset! {});
    }

    #[test]
    fn tagged() {
        assert_tags!("tagged.png", tagset! { "foo", "bar" });
    }

    #[test]
    fn multiple_chunks() {
        assert_tags!("multiple_chunks.png", tagset! { "foo", "bar" });
    }
}
