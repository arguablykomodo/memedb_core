use crate::error::Error;
use crate::reader::{IoResult, Reader};
use crate::xml::{XmlTag, XmlTree};
use crate::TagSet;
use colored::*;
use log::{debug, error, info};
use std::collections::HashSet;
use std::io::Error as IoError;
use std::iter::Peekable;

pub const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
const EOF_CHUNK_TYPE: u8 = 0xD9;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

pub struct JpgReader;
impl Reader for JpgReader {
    fn read_tags(file: &mut impl Iterator<Item = IoResult>) -> Result<TagSet, Error> {
        let mut tags: TagSet = HashSet::new();
        use crate::helpers::log_address::LogAddress;
        let mut file_iterator: Peekable<_> = file.log().peekable();
        let mut chunk_type: u8;
        loop {
            if next!(file_iterator) == 0x0FF {
                chunk_type = next!(file_iterator);
                if next!(file_iterator;peek) == Some(&0xFF) {
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
                    let chunk_data = JpgReader::get_chunk_data(&mut file_iterator)?;

                    // XML inside jpg always start with http
                    if &chunk_data[0..4] != b"http" {
                        continue;
                    }

                    if let Ok(chunk_string) = String::from_utf8(chunk_data) {
                        info!("This is the XML found: '{}'", chunk_string.yellow());
                        tags = JpgReader::parse_xml(&chunk_string)?;
                        break;
                    } else {
                        info!("Chunk-data found couldn't be converted to string");
                        continue;
                    }
                } else {
                    JpgReader::skip_chunk_data(&mut file_iterator)?;
                }
            } else {
                error!("Skipping bytes");
            }
        }
        Ok(tags)
    }
    fn write_tags(
        file_iterator: &mut impl Iterator<Item = IoResult>,
        tags: &TagSet,
    ) -> Result<Vec<u8>, Error> {
        use std::time::SystemTime;
        let t = SystemTime::now(); // Poor's Man benchmark
        let mut bytes: Vec<u8> = SIGNATURE
            .iter()
            .copied()
            .map(Ok)
            .chain(file_iterator)
            .collect::<Result<_, IoError>>()?;
        let mut tags_start: Option<usize> = None; // These 2 hold the addresses of the tag's chunk
        let mut tags_end: Option<usize> = None; //
        let windows = bytes.windows(2); // Iterate in pairs
        for (addr, slice) in windows.enumerate() {
            // Skip until chunk
            if slice[0] != 0xFF {
                continue;
            }
            // Break the loop when the start of another chunk is found
            if slice[1] != 0x00 && tags_start != None {
                info!("Found 0xFFE1 end on {}", addr);
                tags_end = Some(addr);
                break;
            }
            // This checks if tags were found
            if slice[1] == TAGS_CHUNK_TYPE {
                if &bytes[addr + 4..addr + 8] == b"http" {
                    info!("0xFFE1 found on {}", addr);
                    tags_start = Some(addr);
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
        // it is safe to add a new chunk there
        let tags_end = tags_end.unwrap_or_else(|| bytes.len() - 2);
        let tags_start = tags_start.unwrap_or_else(|| bytes.len() - 2);

        let mut tags_bytes: Vec<u8> = vec![0xFF, TAGS_CHUNK_TYPE, 0x00, 0x00];
        tags_bytes.append(&mut JpgReader::create_xml(tags));
        let mut bytes_diff: isize = (tags_end - tags_start) as isize - tags_bytes.len() as isize;
        // Take the existing chunk in the file and resize it to fit the new chunk
        if bytes_diff < 0 {
            loop {
                bytes.insert(tags_start, 0x00);
                bytes_diff += 1;
                if bytes_diff == 0 {
                    break;
                }
            }
        } else {
            loop {
                bytes.remove(tags_start);
                bytes_diff -= 1;
                if bytes_diff == 0 {
                    break;
                }
            }
        }
        // Copy the bytes of the tag's chunk into the file's buffer
        info!("Writting {} ({0:#02X}) bytes", tags_bytes.len());
        for (i, b) in tags_bytes.iter().enumerate() {
            bytes[tags_start + i] = *b;
        }

        use std::convert::TryInto;
        // This tries to convert the chunk's length into a u16 (0-65535), returning an error if it couldn't
        // The -2 is there because otherwise the length would take into count the 0xFFE1
        let tags_bytes_length: u16 = match (tags_bytes.len() - 2).try_into() {
            Ok(v) => v,
            Err(_) => return Err(Error::Parser),
        };
        bytes[tags_start + 3] = (tags_bytes_length & 0xFF) as u8;
        bytes[tags_start + 2] = ((tags_bytes_length >> 8) & 0xFF) as u8;
        debug!("Finished in {:?}", t.elapsed().unwrap());

        Ok(bytes)
    }
}

impl JpgReader {
    fn get_chunk_data(
        mut file_iterator: &mut Peekable<impl Iterator<Item = IoResult>>,
    ) -> Result<Vec<u8>, Error> {
        let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
        let chunk_data: Vec<u8> = file_iterator
            .take(chunk_length)
            .collect::<Result<Vec<u8>, IoError>>()?;
        if chunk_data.len() != chunk_length {
            error!(
                "{}",
                format!(
                    "Error: The data captured is shorter than expected\n{} bytes expected, got {}",
                    chunk_length,
                    chunk_data.len()
                )
                .red()
            );
            next!(file_iterator);
            Err(Error::Parser)
        } else {
            debug!("Read {:#02X?}", &chunk_data[chunk_data.len() - 8..]);
            Ok(chunk_data)
        }
    }

    fn skip_chunk_data(
        mut file_iterator: &mut Peekable<impl Iterator<Item = IoResult>>,
    ) -> Result<(), Error> {
        let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
        for _ in 0..chunk_length {
            next!(file_iterator);
        }
        Ok(())
    }
    fn get_chunk_length(
        file_iterator: &mut Peekable<impl Iterator<Item = IoResult>>,
    ) -> Result<usize, Error> {
        let mut chunk_length = 0x0000;
        chunk_length |= (next!(file_iterator) as usize) << 8;
        chunk_length |= next!(file_iterator) as usize;
        chunk_length -= 2;
        debug!("Req. chunk of {:#04X} bytes", chunk_length);
        Ok(chunk_length)
    }

    fn parse_xml(xml: &str) -> Result<TagSet, Error> {
        let tree = XmlTree::parse(xml.to_string())?;
        let finds = tree.find_elements(|e: &XmlTag| match e.attributes.get("rdf:about") {
            Some(v) => v == KEYWORDS_UUID,
            None => false,
        });
        let mut tags: TagSet = TagSet::new();
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
        let mut tags_string = String::with_capacity(tags.len() * (8 + 10 + 9));
        let mut tags: Vec<&String> = tags.iter().collect();
        tags.sort_unstable();
        for tag in tags {
            tags_string.push_str(&format!("<rdf:li>{}</rdf:li>", tag))
        }
        format!(include_str!("template.xml"), tags = tags_string)
            .bytes()
            .collect()
    }
}

reader_tests!(JpgReader, "jpg");
