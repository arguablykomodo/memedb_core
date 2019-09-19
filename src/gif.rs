use crate::error::Error;
use crate::reader::Reader;
use crate::TagSet;
use std::io::Read;

pub struct GifReader {}

impl GifReader {
    fn get_color_table_size(byte: u8) -> usize {
        if byte >> 7 & 1 == 0 {
            0
        } else {
            let size = byte & 0b00000111;
            3 * 1 << (size + 1)
        }
    }

    fn find_tags(bytes: &Vec<u8>) -> Result<(usize, bool), Error> {
        let mut i: usize = 0;

        // Verify signature
        if bytes[0..6] != *b"GIF89a" {
            return Err(Error::Format);
        }
        i += 6;

        // Get info from descriptor
        let color_table_size = GifReader::get_color_table_size(bytes[10]);
        i += 7;

        // Skip color table
        i += color_table_size;

        loop {
            match bytes[i] {
                // Trailer, signifies end of file
                0x3B => return Ok((i, false)),
                // Extension block
                0x21 => {
                    let label = bytes[i + 1];
                    let size = bytes[i + 2] as usize;
                    let data = &bytes[i + 3..i + 3 + size];
                    i += 3 + size;

                    if label == 0xFF && data == b"MEMETAGS1.0" {
                        return Ok((i, true));
                    }

                    loop {
                        if bytes[i] == 0 {
                            i += 1;
                            break;
                        }
                        let sub_block_size = bytes[i] as usize;
                        i += sub_block_size + 1;
                    }
                }
                // Image Block
                0x2C => {
                    let color_table_size = GifReader::get_color_table_size(bytes[i + 9]);
                    i += 10;

                    i += color_table_size;

                    // Loop through sub-blocks
                    i += 1;
                    loop {
                        if bytes[i] == 0 {
                            i += 1;
                            break;
                        }
                        let sub_block_size = bytes[i] as usize;
                        i += sub_block_size + 1;
                    }
                }
                _ => return Err(Error::Format),
            };
        }
    }
}

impl Reader for GifReader {
    fn read_tags(file: &mut impl Read) -> Result<TagSet, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let (mut i, found) = GifReader::find_tags(&bytes)?;
        let mut tags = TagSet::new();
        if !found {
            return Ok(tags);
        } else {
            loop {
                if bytes[i] == 0 {
                    return Ok(tags);
                }
                let sub_block_size = bytes[i] as usize;
                i += 1;
                tags.insert(String::from_utf8_lossy(&bytes[i..i + sub_block_size]).to_string());
                i += sub_block_size;
            }
        }
    }

    fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let mut tags: Vec<String> = tags.iter().cloned().collect();
        tags.sort_unstable();
        let mut tag_bytes = Vec::new();
        for tag in &mut tags {
            tag_bytes.push(tag.len() as u8);
            tag_bytes.append(&mut tag.as_bytes().into_iter().cloned().collect());
        }
        tag_bytes.push(0);

        let (mut i, found) = GifReader::find_tags(&bytes)?;
        if !found {
            let mut real_bytes = vec![
                0x21, 0xFF, 0x0B, b'M', b'E', b'M', b'E', b'T', b'A', b'G', b'S', b'1', b'.', b'0',
            ];
            real_bytes.append(&mut tag_bytes);
            bytes.splice(i..i, real_bytes);
            return Ok(bytes);
        } else {
            let start = i;
            loop {
                if bytes[i] == 0 {
                    bytes.splice(start..i, tag_bytes);
                    return Ok(bytes);
                }
                let sub_block_size = bytes[i] as usize;
                i += sub_block_size + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_read_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        // mem::discriminant magic is used to compare enums without having to implement PartialEq
        assert_eq!(
            std::mem::discriminant(&GifReader::read_tags(&mut file).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_read_empty() {
        let mut file = File::open("tests/empty.gif").unwrap();
        let tags = TagSet::new();
        assert_eq!(GifReader::read_tags(&mut file).unwrap(), tags);
    }

    #[test]
    fn test_read_tagged() {
        let mut file = File::open("tests/tagged.gif").unwrap();
        let mut tags = TagSet::new();
        tags.insert("foo".to_owned());
        tags.insert("bar".to_owned());
        assert_eq!(GifReader::read_tags(&mut file).unwrap(), tags);
    }

    #[test]
    fn test_write_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        let tags = TagSet::new();
        // mem::discriminant magic is used to compare enums without having to implement PartialEq
        assert_eq!(
            std::mem::discriminant(&GifReader::write_tags(&mut file, &tags).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_write_empty() {
        let mut file = File::open("tests/empty.gif").unwrap();

        let mut tags = TagSet::new();
        tags.insert("foo".to_owned());
        tags.insert("bar".to_owned());

        let result_bytes = GifReader::write_tags(&mut file, &tags).unwrap();

        let mut test = File::open("tests/tagged.gif").unwrap();
        let mut test_bytes = Vec::new();
        test.read_to_end(&mut test_bytes).unwrap();

        assert_eq!(result_bytes, test_bytes);
    }

    #[test]
    fn test_write_tagged() {
        let mut file = File::open("tests/tagged.gif").unwrap();

        let tags = TagSet::new();

        let result_bytes = GifReader::write_tags(&mut file, &tags).unwrap();

        let mut test = File::open("tests/untagged.gif").unwrap();
        let mut test_bytes = Vec::new();
        test.read_to_end(&mut test_bytes).unwrap();

        assert_eq!(result_bytes, test_bytes);
    }
}
