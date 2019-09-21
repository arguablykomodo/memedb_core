use crate::error::Error;
use crate::reader::Reader;
use crate::TagSet;
use crc::crc32;
use std::io::{BufReader, Read};

macro_rules! next {
    ($i:ident) => {
        $i.next().ok_or(Error::EOF)??
    };
}

const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct PngReader {}

impl Reader for PngReader {
    fn read_tags(file: &mut impl Read) -> Result<TagSet, Error> {
        let mut bytes = BufReader::new(file).bytes();

        for byte in SIGNATURE.iter() {
            if *byte != next!(bytes) {
                return Err(Error::Format);
            }
        }

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

    fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        if bytes[0..SIGNATURE.len()] != *SIGNATURE {
            return Err(Error::Format);
        };

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
        new_chunk.append(&mut vec![b'm', b'e', b'M', b'e']);
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

    #[test]
    fn test_read_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        // mem::discriminant magic is used to compare enums without having to implement PartialEq
        assert_eq!(
            std::mem::discriminant(&PngReader::read_tags(&mut file).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_read_empty() {
        let mut file = File::open("tests/empty.png").unwrap();
        let tags = TagSet::new();
        assert_eq!(PngReader::read_tags(&mut file).unwrap(), tags);
    }

    #[test]
    fn test_read_tagged() {
        let mut file = File::open("tests/tagged.png").unwrap();
        let mut tags = TagSet::new();
        tags.insert("foo".to_owned());
        tags.insert("bar".to_owned());
        assert_eq!(PngReader::read_tags(&mut file).unwrap(), tags);
    }

    #[test]
    fn test_write_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        let tags = TagSet::new();
        // mem::discriminant magic is used to compare enums without having to implement PartialEq
        assert_eq!(
            std::mem::discriminant(&PngReader::write_tags(&mut file, &tags).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_write_empty() {
        let mut file = File::open("tests/empty.png").unwrap();

        let mut tags = TagSet::new();
        tags.insert("foo".to_owned());
        tags.insert("bar".to_owned());

        let result_bytes = PngReader::write_tags(&mut file, &tags).unwrap();

        let mut test = File::open("tests/tagged.png").unwrap();
        let mut test_bytes = Vec::new();
        test.read_to_end(&mut test_bytes).unwrap();

        assert_eq!(result_bytes, test_bytes);
    }

    #[test]
    fn test_write_tagged() {
        let mut file = File::open("tests/tagged.png").unwrap();

        let tags = TagSet::new();

        let result_bytes = PngReader::write_tags(&mut file, &tags).unwrap();

        let mut test = File::open("tests/untagged.png").unwrap();
        let mut test_bytes = Vec::new();
        test.read_to_end(&mut test_bytes).unwrap();

        assert_eq!(result_bytes, test_bytes);
    }
}
