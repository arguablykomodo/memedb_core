use crate::error::Error;
use crate::reader::Reader;
use crate::xml::{XmlTag, XmlTree};
use std::collections::HashSet;
use std::io::{Bytes, Read};

/* Explanation:
  1. JpgReader verifies the signature

  [loop]
  2. JpgReader tries to find a chunk
    a. If the value in the var "byte" is an 0xFF then
        that means the byte it has now is the chunk-type.
        Finally try to guess if this chunk has data by going to [3]
    b. if it is not then that probably means the
        reader is inside a data chunk. JpgReader discards
        the value, reads a new one and then goes to [2]

  3. JpgReader reads the next byte
    a. If it is an 0xFF, then this is the start of another chunk.
      Save this value in the var "byte", process this chunk by going to [4]
    b. Otherwise, it reads the next byte and appends them like this: FIRST<<8+SECOND (This is a 16 bit big-endian)

  4. Finally process the chunk
    a. If the chunk type is 0xEn, then tags may be found here
    b. If the chunk type is 0xD9, then this is the end of the file. The break the loop and finish by doing [5]
    c. Any other type of chunk is skipped

  5. Finish and return the tags
*/
const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

enum JpgReaderState {
  WatingChunkType,
  RecordingChunkData(u16), //Data length
  WatingChunkLength,
  ProcessChunk,
}
pub struct JpgReader;
impl Reader for JpgReader {
  fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error> {
    let mut tags: HashSet<String> = HashSet::new();
    let mut bytes: std::iter::Peekable<_> = file.bytes().peekable();
    for byte in SIGNATURE.iter() {
      if *byte != bytes.next().unwrap()? {
        return Err(Error::UnknownFormat);
      }
    }
    let mut chunk_type = 0x00;
    let mut reader_state = JpgReaderState::WatingChunkType;
    let mut byte = 0xFF;
    let mut last_byte = *SIGNATURE.last().unwrap();
    let mut chunk_data: Vec<u8> = vec![];
    // Main loop, iterate through all the bytes until EOF
    loop {
      match reader_state {
        JpgReaderState::WatingChunkType if last_byte == 0xFF => {
          chunk_type = byte;
          // If we are in the end of the file, we manually set it to finish the parsing
          reader_state = if chunk_type == 0xD9 {
            JpgReaderState::ProcessChunk
          } else {
            JpgReaderState::WatingChunkLength
          };
        }
        JpgReaderState::WatingChunkType => {
          // This justs discards bytes that the parser couldn't understand
          last_byte = byte;
          byte = bytes.next().unwrap()?;
        }
        JpgReaderState::WatingChunkLength => {
          let next_byte = bytes.next().unwrap()?;
          // Detect if the next bytes the start of another chunk or the length of the data
          if next_byte == 0xFF {
            reader_state = JpgReaderState::RecordingChunkData(0);
            byte = next_byte;
          } else {
            last_byte = next_byte;
            byte = bytes.next().unwrap()?;
            let chunk_length = ((last_byte as u16) << 8) | (byte as u16);
            reader_state = JpgReaderState::RecordingChunkData(chunk_length);
          }
        }
        JpgReaderState::RecordingChunkData(length) => {
          let length = if length > 0 { length - 2 } else { 0 };
          println!("Last recorded data:\n{:02X?}", chunk_data);
          chunk_data = Vec::with_capacity(length as usize);
          println!("Recording chunk of {0:02X?}/{0} len", length);
          for i in 0..length {
            match bytes.next().unwrap() {
              Ok(b) => {
                if b == 0xFF {
                  println!("Found 0xFF at {:02X}", i);
                  let skipped: u8 = bytes.next().expect("Error reading lol")?;
                  println!(
                    "This value was skipped (Must be a 0 or 0xFF): {:02X}",
                    skipped
                  );
                  assert!(skipped == 0xFF || skipped == 0x00);
                  chunk_data.push(b);
                  if skipped == 0xFF {
                    chunk_data.push(skipped);
                  }
                } else {
                  chunk_data.push(b);
                }
              }
              Err(e) => {
                println!("Error reading. Read upto {} bytes from {}", i, length);
                println!("Failed with error {:#?}", e);
                panic!(e);
              }
            };
          }
          last_byte = *chunk_data.last().unwrap_or(&last_byte);
          reader_state = JpgReaderState::ProcessChunk;
        }
        JpgReaderState::ProcessChunk => {
          if chunk_type >= 0xE0 && chunk_type <= 0xEF {
            if chunk_type == TAGS_CHUNK_TYPE {
              // Tags may be found here!
              match std::str::from_utf8(&chunk_data) {
                Ok(string) => tags = JpgReader::parse_tags(string)?,
                Err(_) => {} // Not an XML string
              }
            }
          } else if chunk_type == 0xD9 {
            break;
          };
          reader_state = JpgReaderState::WatingChunkType;
          byte = bytes.next().unwrap()?;
        }
      }
    }
    Ok(tags)
  }
  fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
    unimplemented!("Sorry dude, I can't do that yet");
  }
}
impl JpgReader {
  fn parse_tags(xml: &str) -> Result<HashSet<String>, Error> {
    let tree = XmlTree::parse(xml.to_string())?;
    let finds = tree.find_elements(|e: &XmlTag| match e.attributes.get("rdf:about") {
      Some(v) => v == KEYWORDS_UUID,
      None => false,
    });
    let mut tags: HashSet<String> = HashSet::new();
    for i in &finds {
      tree.traverse_map(
        *i,
        |tag: &XmlTag, tags: &mut HashSet<String>| {
          if tag.name == "rdf:li" {
            match &tag.value {
              Some(value) => {
                tags.insert(value.clone());
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
}