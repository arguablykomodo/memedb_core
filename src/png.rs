use crate::error::Error;
use crate::reader::{IoResult, Reader};
use crate::TagSet;
use crc::crc32;
use std::io::{BufRead, Bytes, Read, Error as IoError};

pub const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct PngReader {}

impl Reader for PngReader {
    fn read_tags(bytes: &mut impl Iterator<Item = IoResult>) -> Result<TagSet, Error> {
        loop {
            let mut length = 0u32;
            for _ in 0..4 {
                length = (length << 8) + u32::from(next!(bytes));
            }

            let mut chunk_type = [0; 4];
            for byte in &mut chunk_type {
                *byte = next!(bytes);
            }

            match &chunk_type {
                b"IEND" => return Ok(TagSet::new()),
                b"meMe" => {
                    let mut tags = TagSet::new();
                    let mut text = String::new();

                    for _ in 0..length {
                        let byte = next!(bytes);
                        if byte == b';' {
                            tags.insert(text);
                            text = String::new();
                        } else {
                            text.push(byte as char);
                        }
                    }
                    return Ok(tags);
                }
                _ => {
                    for _ in 0..length {
                        next!(bytes);
                    }
                }
            }

            // Every chunk ends with a 4 byte long checksum
            for _ in 0..4 {
                next!(bytes);
            }
        }
    }

    fn write_tags(
        file: &mut impl Iterator<Item = IoResult>,
        tags: &TagSet,
    ) -> Result<Vec<u8>, Error> {
        let mut bytes: Vec<u8> = SIGNATURE
            .iter()
            .copied()
            .map(Ok)
            .chain(file)
            .collect::<Result<_, IoError>>()?;

        let mut tags: Vec<String> = tags.iter().cloned().collect();
        tags.sort_unstable();
        let mut tags: Vec<u8> = tags
            .iter()
            .cloned()
            .map(|t| (t + ";").into_bytes())
            .flatten()
            .collect();

        let mut chunk_length = Vec::new();
        for i in 0..4 {
            chunk_length.push((tags.len() >> ((3 - i) * 8)) as u8);
        }

        let mut new_chunk = Vec::new();
        new_chunk.append(&mut b"meMe".to_vec());
        new_chunk.append(&mut tags);
        let checksum = crc32::checksum_ieee(&new_chunk);
        new_chunk.append(&mut vec![
            (checksum >> 24 & 0xFF) as u8,
            (checksum >> 16 & 0xFF) as u8,
            (checksum >> 8 & 0xFF) as u8,
            (checksum & 0xFF) as u8,
        ]);
        new_chunk.splice(0..0, chunk_length);

        let mut i = SIGNATURE.len();
        loop {
            let length = bytes[i..i + 4]
                .iter()
                .enumerate()
                .fold(0, |acc, (i, b)| (acc + u32::from(*b)) << ((3 - i) * 8));
            i += 4;

            // We do this magic so that we dont borrow bytes twice
            let (is_meme, is_end) = {
                let chunk_type = &bytes[i..i + 4];
                (chunk_type == *b"meMe", chunk_type == *b"IEND")
            };
            i += 4;

            // If there is already a meMe chunk, we replace it
            if is_meme {
                bytes.splice(i - 8..i + (length as usize) + 4, new_chunk);
                break;
            }

            // If there is no meMe chunk already, we put one at the end
            if is_end {
                bytes.splice(i - 8..i - 8, new_chunk);
                break;
            }

            // Every chunk ends with a 4 byte long checksum
            i += (length as usize) + 4;
        }

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    // Cool program for checking CRCs:
    // http://www.libpng.org/pub/png/apps/pngcheck.html

    use super::*;
    use std::fs::File;
    use std::io::{BufReader};

    #[test]
    fn test_read_empty() {
        let tags = TagSet::new();
        assert_eq!(
            PngReader::read_tags(&mut open_file!("tests/empty.png", SIGNATURE.len())).unwrap(),
            tags
        );
    }

    #[test]
    fn test_read_tagged() {
        let tags: TagSet = tagset! {"foo","bar"};
        assert_eq!(
            PngReader::read_tags(&mut open_file!("tests/tagged.png", SIGNATURE.len())).unwrap(),
            tags
        );
    }

    #[test]
    fn test_write_empty() {
        let mut empty = open_file!("tests/empty.png", SIGNATURE.len());
        let tags: TagSet = tagset! {"foo","bar"};
        let empty_tagged_bytes: Vec<u8> =
            PngReader::write_tags(&mut empty, &tags).expect("Error in write_tags");
        let tagged = open_file!("tests/tagged.png", 0);
        let tagged_bytes: Vec<u8> = tagged
            .collect::<Result<Vec<u8>, IoError>>()
            .expect("IO error");
        assert_eq!(tagged_bytes, empty_tagged_bytes);
    }

    #[test]
    fn test_write_tagged() {
        let tags = tagset! {};

        let mut tagme = open_file!("tests/tagged.png", SIGNATURE.len());
        let tagme_bytes = PngReader::write_tags(&mut tagme, &tags).unwrap();

        let mut untagged = open_file!("tests/untagged.png", 0);
        let untagged_bytes: Vec<u8> = untagged
            .collect::<Result<Vec<u8>, IoError>>()
            .expect("IO error");

        assert_eq!(tagme_bytes, untagged_bytes);
    }
}
