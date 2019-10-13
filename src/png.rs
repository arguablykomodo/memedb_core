use crate::error::Error;
use crate::reader::{IoResult, Reader};
use crate::TagSet;
use crc::crc32;
use std::io::{BufRead, Bytes, Read, Error as IoError};
use crc::crc32::checksum_ieee;
use std::io;

macro_rules! compose {
    ($i:expr) => {
        (u32::from($i[0]) << 24)
            + (u32::from($i[1]) << 16)
            + (u32::from($i[2]) << 8)
            + u32::from($i[3])
    };
    ($i:ident; iter) => {
        (u32::from(next!($i)) << 24)
            + (u32::from(next!($i)) << 16)
            + (u32::from(next!($i)) << 8)
            + u32::from(next!($i))
    };
}

macro_rules! decompose {
    ($i:expr) => {{
        let n = $i;
        [
            (n >> 24 & 0xFF) as u8,
            (n >> 16 & 0xFF) as u8,
            (n >> 8 & 0xFF) as u8,
            (n & 0xFF) as u8,
        ]
    }};
}

pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct PngReader {}

impl Reader for PngReader {
    fn read_tags(bytes: &mut impl Iterator<Item = IoResult>) -> Result<TagSet, Error> {
        loop {
            let length = compose!(bytes; iter);
            let chunk_type = &[next!(bytes), next!(bytes), next!(bytes), next!(bytes)];

            match chunk_type {
                b"IEND" => return Ok(tagset! {}),
                b"meMe" => {
                    let mut tags = TagSet::new();
                    let mut tag = String::new();
                    for _ in 0..length {
                        match next!(bytes) {
                            b';' => {
                                tags.insert(tag);
                                tag = String::new();
                            }
                            byte => tag.push(byte as char),
                        };
                    }
                    return Ok(tags);
                }
                _ => (),
            }

            for _ in 0..length + 4 {
                next!(bytes);
            }
        }
    }

    fn write_tags(
        bytes: &mut impl Iterator<Item = IoResult>,
        tags: &TagSet,
    ) -> Result<Vec<u8>, Error> {
        let mut tags: Vec<&String> = tags.iter().collect();
        tags.sort_unstable();
        let mut tags: Vec<u8> = tags
            .iter()
            .map(|&t| (t.to_owned() + ";").into_bytes())
            .flatten()
            .collect();

        let mut chunk = Vec::new();
        chunk.extend(&decompose!(tags.len()));
        chunk.extend(b"meMe");
        chunk.append(&mut tags);
        chunk.extend(&decompose!(checksum_ieee(&chunk[4..])));

        let mut bytes = bytes.collect::<Result<Vec<u8>, io::Error>>()?;
        let mut i = 0;
        loop {
            let length = compose!(&bytes[i..i + 4]);
            i += 4;
            let chunk_type = &bytes[i..i + 4];
            i += 4;

            match chunk_type {
                b"meMe" => {
                    bytes.splice(i - 8..i + (length as usize) + 4, chunk);
                    return Ok(bytes);
                }
                b"IEND" => {
                    bytes.splice(i - 8..i - 8, chunk);
                    return Ok(bytes);
                }
                _ => (),
            }

            i += length as usize + 4;
        }
    }
}

// CRC verifier:
// http://www.libpng.org/pub/png/apps/pngcheck.html

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader};

    #[test]
    fn test_read_empty() {
        let mut file = open_file!("tests/empty.png", SIGNATURE.len());
        let result = PngReader::read_tags(&mut file).unwrap();
        assert_eq!(result, tagset! {});
    }

    #[test]
    fn test_read_tagged() {
        let mut file = open_file!("tests/tagged.png", SIGNATURE.len());
        let result = PngReader::read_tags(&mut file).unwrap();
        assert_eq!(result, tagset! {"foo", "bar"});
    }

    #[test]
    fn test_write_empty() {
        let mut empty = open_file!("tests/empty.png", SIGNATURE.len());
        let result = PngReader::write_tags(&mut empty, &tagset! {"foo", "bar"}).unwrap();
        let tagged = open_file!("tests/tagged.png", SIGNATURE.len())
            .map(|b| b.unwrap())
            .collect::<Vec<u8>>();
        assert_eq!(result, tagged);
    }

    #[test]
    fn test_write_tagged() {
        let mut tagged = open_file!("tests/tagged.png", SIGNATURE.len());
        let result = PngReader::write_tags(&mut tagged, &tagset! {}).unwrap();
        let empty = open_file!("tests/untagged.png", SIGNATURE.len())
            .map(|b| b.unwrap())
            .collect::<Vec<u8>>();
        assert_eq!(result, empty);
    }
}
