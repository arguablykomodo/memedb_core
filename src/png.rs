use crate::error::Error;
use crate::reader::{IoResult, Reader};
use crate::TagSet;
use crc::crc32::checksum_ieee;
use log::{debug, info};
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
            debug!(
                "Found chunk {:?}, length: {}",
                String::from_utf8_lossy(chunk_type),
                length
            );

            match chunk_type {
                b"IEND" => {
                    info!("Reached end and no tags in sight");
                    return Ok(TagSet::new());
                }
                b"meMe" => {
                    info!("Found tags, now reading");
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
        file: &mut impl Iterator<Item = IoResult>,
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

        let mut bytes = Vec::new();
        bytes.extend(SIGNATURE);
        bytes.append(&mut file.collect::<Result<Vec<u8>, io::Error>>()?);
        let mut i = SIGNATURE.len();
        loop {
            let length = compose!(&bytes[i..i + 4]);
            i += 4;
            let chunk_type = &bytes[i..i + 4];
            i += 4;

            debug!(
                "Found chunk {:?}, length: {}",
                String::from_utf8_lossy(chunk_type),
                length
            );

            match chunk_type {
                b"meMe" => {
                    info!("Found existing tags at {:X}, going to replace", i);
                    bytes.splice(i - 8..i + (length as usize) + 4, chunk);
                    return Ok(bytes);
                }
                b"IEND" => {
                    info!("didn't find tags, appending at {:X}", i);
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
reader_tests!(PngReader, "png");
