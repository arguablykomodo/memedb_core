use crate::error::Error;
use crate::log::{debug, error, info};
use crate::reader::Reader;
use crate::xml::{XmlTag, XmlTree};
use crate::TagSet;
use colored::*;
use std::collections::HashSet;
use std::io::Read;
use std::iter::Peekable;

const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
const EOF_CHUNK_TYPE: u8 = 0xD9;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

/* #region Debugging tools */
#[cfg(logAddresses)]
mod log_address {
    use std::iter::*;

    pub trait LogAddress<Item, I: Iterator<Item = Item>> {
        fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item>;
    }

    impl<Item, I> LogAddress<Item, I> for I
    where
        I: Iterator<Item = Item>,
    {
        fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item> {
            self.enumerate().map(&|(a, v)| {
                debug!("Address: 0x{:06X?}", a);
                v
            })
        }
    }
}
#[cfg(not(logAddresses))]
mod log_address {
    pub trait LogAddress<I: Iterator> {
        fn log<'a>(self) -> I;
    }
    impl<I> LogAddress<I> for I
    where
        I: Iterator,
    {
        fn log<'a>(self) -> I {
            self
        }
    }
}
/* #endregion */

macro_rules! read {
    ($i:ident) => {
        match $i.next() {
            Some(r) => r.map_err(|err| err.into()),
            None => Err(Error::EOF),
        }
    };
    ($i:ident; peek) => {
        match $i.peek() {
            Some(r) => match r {
                Ok(v) => Some(*v),
                Err(_) => None,
            },
            None => None,
        }
    };
}

pub struct JpgReader;
impl Reader for JpgReader {
    fn read_tags(file: &mut impl Read) -> Result<TagSet, Error> {
        let mut tags: TagSet = HashSet::new();
        use log_address::LogAddress;
        let mut file_iterator: Peekable<_> = file.bytes().log().peekable();
        JpgReader::verify_signature(&mut file_iterator)?;
        let mut chunk_type: u8;
        loop {
            /* Loops in rust have a bug where they consume variables in spite of borrowing them
            making them unusable in the next iteration (thus failing even to compile)
            Just declaring a dumb var and pointing it to the desired variable makes it usable in all the iterations */
            let mut file_iterator_ref = &mut file_iterator;
            let peeked = match read!(file_iterator_ref) {
                Ok(v) => v,
                Err(_) => break,
            };
            if peeked == 0xFF {
                chunk_type = read!(file_iterator_ref)?;
                if read!(file_iterator_ref;peek) == Some(0xFF) {
                    info!("Peeked the start of another chunk");
                    continue;
                }
                info!("Chunk type: {:#02X?}", chunk_type);
                if chunk_type == 0x00 {
                    continue;
                } else if chunk_type == EOF_CHUNK_TYPE {
                    debug!("{}", "EOF".green());
                    break;
                } else if chunk_type == TAGS_CHUNK_TYPE {
                    let chunk_data = JpgReader::get_chunk_data(&mut file_iterator_ref)?;
                    let chunk_string;

                    if chunk_data[0] != 0x68 {
                        continue;
                    }

                    chunk_string = match String::from_utf8(chunk_data) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Chunk data wasn't an XML (Failed with error {:#X?})", e);
                            continue;
                        }
                    };
                    info!("This is the XML found: '{}'", chunk_string.yellow());
                    match JpgReader::parse_xml(&chunk_string) {
                        Ok(t) => {
                            tags = t;
                            break;
                        }
                        Err(e) => eprintln!("XML parser error {}", format!("{:?}", e).red()),
                    }
                } else {
                    JpgReader::skip_chunk_data(&mut file_iterator_ref)?;
                }
            } else {
                error!("Skipping bytes");
            }
        }
        Ok(tags)
    }
    fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error> {
        let mut file_iterator: _ = file.bytes();
        JpgReader::verify_signature(&mut file_iterator)?;
        use std::time::SystemTime;
        let t = SystemTime::now(); // Poor's Man benchmark
        let mut bytes: Vec<u8> = SIGNATURE.iter().map(|v| *v).collect::<Vec<u8>>();
        for byte in file_iterator {
            bytes.push(byte?);
        }
        let mut tags_address_start: Option<usize> = None; // These 2 hold the addresses of the tag's chunk
        let mut tags_address_end: Option<usize> = None; //
        let windows = bytes.windows(2); // Iterate in pairs
        for (addr, slice) in windows.enumerate() {
            // Skip until chunk
            if slice[0] != 0xFF {
                continue;
            }
            // slice[0] == 0xFF
            // When found another chunk, break if the tags were found
            if slice[1] != 0x00 && tags_address_start != None {
                info!("Found 0xFFE1 end on {}", addr);
                tags_address_end = Some(addr);
                break;
            }
            // This checks if tags were found
            if slice[1] == TAGS_CHUNK_TYPE {
                if &bytes[addr + 4..addr + 8] == &['h' as u8, 't' as u8, 't' as u8, 'p' as u8] {
                    info!("0xFFE1 found on {}", addr);
                    tags_address_start = Some(addr);
                } else {
                    info!(
                        "On {:#06X?} found {:#04X?}",
                        addr,
                        &bytes[addr + 4..addr + 8]
                    );
                }
            }
        }
        // If no chunk was found, make the vars point to the end of the file
        if tags_address_start == None {
            info!("This image contains no tags");
            tags_address_start = Some(bytes.len() - 2);
            tags_address_end = Some(bytes.len() - 2);
        }
        let mut tags_bytes: Vec<u8> = vec![0xFF, TAGS_CHUNK_TYPE, 0x00, 0x00];
        tags_bytes.append(&mut JpgReader::create_xml(tags));
        let mut bytes_diff: isize = (tags_address_end.unwrap() - tags_address_start.unwrap())
            as isize
            - tags_bytes.len() as isize;
        // Take the existing chunk in the file and resize it to fit the new chunk
        if bytes_diff < 0 {
            loop {
                bytes.insert(tags_address_start.unwrap(), 0x00);
                bytes_diff += 1;
                if bytes_diff == 0 {
                    break;
                }
            }
        } else if bytes_diff > tags_bytes.len() as isize {
            loop {
                bytes.remove(tags_address_start.unwrap());
                bytes_diff -= 1;
                if bytes_diff == 0 {
                    break;
                }
            }
        }
        // Copy the bytes of the tag's chunk into the file's buffer
        info!("Writting {} ({0:#02X}) bytes", tags_bytes.len());
        for (i, b) in tags_bytes.iter().enumerate() {
            bytes[tags_address_start.unwrap() + i] = *b;
        }

        use std::convert::TryInto;
        // This tries to convert the chunk's length into a u16 (0-65535), returning an error if it couldn't
        // The -2 is there because otherwise the length would take into count the 0xFFE1
        let tags_bytes_length: u16 = match (tags_bytes.len() - 2).try_into() {
            Ok(v) => v,
            Err(_) => return Err(Error::Format),
        };
        bytes[tags_address_start.unwrap() + 3] = (tags_bytes_length & 0xFF) as u8;
        bytes[tags_address_start.unwrap() + 2] = ((tags_bytes_length >> 8) & 0xFF) as u8;
        debug!("Finished in {:?}", t.elapsed().unwrap());

        return Ok(bytes);
    }
}
impl JpgReader {
    fn get_chunk_data(
        mut file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
    ) -> Result<Vec<u8>, Error> {
        let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
        let chunk_data: Vec<u8> = file_iterator
            .take(chunk_length)
            .map(|v| v.unwrap())
            .collect();
        if chunk_data.len() != chunk_length {
            eprintln!(
                "{}",
                format!(
                    "Error: The data captured is shorter than expected\n{} bytes expected, got {}",
                    chunk_length,
                    chunk_data.len()
                )
                .red()
            );
            return match read!(file_iterator) {
                Ok(_) => Err(Error::Format),
                Err(e) => Err(e),
            };
        } else {
            debug!("Read {:#02X?}", &chunk_data[chunk_data.len() - 8..]);
            return Ok(chunk_data);
        }
    }
    fn skip_chunk_data(
        mut file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
    ) -> Result<(), Error> {
        let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
        for _ in 0..chunk_length {
            match file_iterator.next() {
                Some(v) => v?,
                None => return Err(Error::EOF),
            };
        }
        Ok(())
    }
    fn get_chunk_length(
        file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
    ) -> Result<usize, Error> {
        let mut chunk_length = 0x0000;
        chunk_length |= (read!(file_iterator)? as usize) << 8;
        chunk_length |= read!(file_iterator)? as usize;
        chunk_length -= 2;
        debug!("Req. chunk of {:#04X} bytes", chunk_length);
        return Ok(chunk_length);
    }
    fn verify_signature(
        file_iterator: &mut impl Iterator<Item = Result<u8, std::io::Error>>,
    ) -> Result<(), Error> {
        for bytes in SIGNATURE.iter().zip(file_iterator) {
            // Iterate first bytes of the file and SIGNATURE at the same time
            if *bytes.0 != bytes.1? {
                // The for returns a tuple: (&u8,Result<u8,E>), the 1st value is the signature, the other is the file
                info!("Signature checking failed.");
                return Err(Error::Format);
            }
        }
        Ok(())
    }
    fn parse_xml(xml: &str) -> Result<TagSet, Error> {
        let tree = XmlTree::parse(xml.to_string())?;
        let finds = tree.find_elements(|e: &XmlTag| match e.attributes.get("rdf:about") {
            Some(v) => v == KEYWORDS_UUID,
            None => false,
        });
        let mut tags: TagSet = HashSet::new();
        for i in &finds {
            tree.traverse_map(
                *i,
                |tag: &XmlTag, tags: &mut TagSet| {
                    if tag.name == "rdf:li" {
                        match &tag.value {
                            Some(value) => {
                                tags.insert(value.to_string());
                            }
                            None => {}
                        };
                    }
                    tags
                },
                &mut tags,
            );
        }
        Ok(tags)
    }
    fn create_xml(tags: &TagSet) -> Vec<u8> {
        let mut tags_string = String::with_capacity(tags.len() * (8 + 9));
        for tag in tags {
            tags_string.push_str(&format!("<rdf:li>{}</rdf:li>", tag))
        }
        format!(include_str!("template.xml"), tags = tags_string)
            .bytes()
            .collect::<Vec<u8>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_read_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        assert_eq!(
            std::mem::discriminant(&JpgReader::read_tags(&mut file).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_read_empty() {
        let mut file = File::open("tests/empty.jpg").unwrap();
        let tags: Result<TagSet, Error> = JpgReader::read_tags(&mut file);
        assert!(tags.is_ok());
        let tags: TagSet = tags.unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_read_tagged() {
        let mut file = File::open("tests/tagged.jpg").unwrap();
        let tags: Result<TagSet, Error> = JpgReader::read_tags(&mut file);
        assert!(tags.is_ok());
        let tags: TagSet = tags.unwrap();
        assert!(tags.contains("pepe"));
    }

    #[test]
    fn test_write_invalid() {
        let mut file = File::open("tests/invalid").unwrap();
        let tags = TagSet::new();
        assert_eq!(
            std::mem::discriminant(&JpgReader::write_tags(&mut file, &tags).unwrap_err()),
            std::mem::discriminant(&Error::Format)
        );
    }

    #[test]
    fn test_write_empty() {
        let mut empty = File::open("tests/empty.jpg").unwrap();
        let mut tags = TagSet::new();
        tags.insert("pepe".to_string());
        let empty_bytes = JpgReader::write_tags(&mut empty, &tags).unwrap();

        let mut tagged = File::open("tests/tagged.jpg").unwrap();
        let mut tagged_bytes = Vec::new();
        tagged.read_to_end(&mut tagged_bytes).unwrap();

        assert_eq!(empty_bytes, tagged_bytes);
    }

    #[test]
    fn test_write_tagged() {
        let mut file = File::open("tests/tagged.jpg").unwrap();
        let tags = TagSet::new();
        let result_bytes = JpgReader::write_tags(&mut file, &tags).unwrap();
        let mut untagged = File::open("tests/untagged.jpg").unwrap();
        let mut untagged_bytes = Vec::new();
        untagged.read_to_end(&mut untagged_bytes).unwrap();

        assert_eq!(result_bytes, untagged_bytes);
    }
}
