use crate::error::Error;
use crate::reader::Reader;

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
    b. Otherwise, it reads the next byte and appends them like this: (FIRST<<8 | SECOND)

  4. Finally process the chunk
    a. If the chunk type is 0xEn, then tags may be found here
    b. If the chunk type is 0xD9, then this is the end of the file. The break the loop and finish by doing [5]
    c. Any other type of chunk is skipped

  5. Finish and return the tags
*/
enum JpgReaderState {
  watingChunkType,
  recordChunkData(u16), //Data length
  watingChunkLength,
  processChunk,
}
pub struct JpgReader;
const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;

impl Reader for JpgReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error> {
    let mut tags: HashSet<String> = HashSet::new();
    for byte in SIGNATURE.iter() {
      if *byte != JpgReader::next(bytes)? {
        return Err(Error::UnknownFormat);
      }
    }

    let mut chunk_type = 0x00;
    let mut reader_state = JpgReaderState::watingChunkType;
    let mut last_byte = *SIGNATURE.last().unwrap();
    let mut byte = JpgReader::next(bytes)?;
    let mut chunk_data: Vec<u8> = vec![];
    loop {
      /* if 0xFF == byte {
        let chunk_type = JpgReader::next(bytes)?;
        let mut chunk_size: u16;
        let mut is_tags_chunk = false;
        println!("------------------------------------------");
        println!("Byte found: {}", chunk_type);
        match chunk_type {
          0xD8 => {
            println!("Found chunk of 0xD8");
            chunk_size = 0;
          }
          0xC0 => {
            println!("Found chunk of 0xC0");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xC2 => {
            println!("Found chunk of 0xC2");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xC4 => {
            println!("Found chunk of 0xC4");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xDA => {
            println!("Found chunk of 0xDA\nThe images is stored here");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xDB => {
            println!("Found chunk of 0xDB");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xDD => {
            println!("Found chunk of 0xDD");
            chunk_size = 4;
          }
          n @ 0xD0...0xD7 => {
            println!("Found chunk of 0x{:02X}", n);
            chunk_size = 0;
          }
          n @ 0xE0...0xEF => {
            println!("Found chunk of 0x{:02X}", n);
            if n == 0xE1 {
              println!("Tags may be found here!");
              is_tags_chunk = true;
            }
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xFE => {
            println!("Found chunk of 0xFE");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xD9 => {
            println!("Found chunk of 0xD9");
            chunk_size = 0;
            println!("Finished");
            break;
          }
          _ => {
            let bytes_around: Vec<u8> = bytes
              .take(255)
              .take_while(|b| b.is_ok())
              .map(|b| b.unwrap())
              .collect();
            println!(
              "Unkown chunk type {:02X} \nNext 255 bytes: {:02X?}",
              chunk_type, bytes_around
            );
            return Err(Error::WrongFormat);
          }
        println!(
          "Skiping {} 0x{0:04X} bytes\n(Actually {} 0x{1:04X}) bytes",
          chunk_size,
          chunk_size - 2
        );
        chunk_size = chunk_size - 2;
        let mut skipped_bytes = Vec::with_capacity(chunk_size as usize);
        while chunk_size > 0 {
          let byte = JpgReader::next(bytes)?;
          skipped_bytes.push(byte);
          chunk_size -= 1;
        match std::str::from_utf8(&skipped_bytes) {
          Ok(v) if is_tags_chunk => {
            println!(
              "This was read on an APPn marker: '{}'",
              &v.split_whitespace()
                .map(|v: &str| { v.to_string() + " " })
                .collect::<String>()
            );
          }
          Err(_) | Ok(_) => {
            println!(
              "These bytes were skipped {:02X?}",
              //skipped_bytes.iter().take(10).collect::<Vec<&u8>>()
              skipped_bytes
            );
          }
        };
      } else {
        let bytes_around: Vec<u8> = bytes
          .take(255)
          .take_while(|b| b.is_ok())
          .map(|b| b.unwrap())
          .collect();
        println!(
          "Error: Expected FF read {:02X}\nNext 255 bytes: {:02X?}",
          byte, bytes_around
        );
        return Err(Error::WrongFormat);
      } */
      match reader_state {
        JpgReaderState::watingChunkType if last_byte == 0xFF => {
          println!("Retrieving chunk type");
          chunk_type = byte;
          println!("Chunk type: {:02X}", chunk_type);
          // If we are in the end of the file, we manually set it to finish the parsing
          reader_state = if chunk_type == 0xD9 {
            JpgReaderState::processChunk
          } else {
            JpgReaderState::watingChunkLength
          };
        }
        JpgReaderState::watingChunkType => {
          println!("Waiting chunk type...");
          last_byte = byte;
          byte = JpgReader::next(bytes)?;
          println!("Read byte: {:02X}{:02X?}", last_byte, byte);
        }
        JpgReaderState::watingChunkLength => {
          println!("Getting chunk length");
          let next_byte = JpgReader::next(bytes)?;
          if next_byte == 0xFF {
            println!("0-length chunk");
            reader_state = JpgReaderState::recordChunkData(0);
            byte = next_byte;
          } else {
            last_byte = next_byte;
            byte = JpgReader::next(bytes)?;
            let chunk_length = ((last_byte as u16) << 8) | (byte as u16);
            println!("Chunk length: {:04X}", chunk_length);
            reader_state = JpgReaderState::recordChunkData(chunk_length);
          }
        }
        JpgReaderState::recordChunkData(length) => {
          let length = if length > 0 { length - 2 } else { 0 };
          println!("Storing {:04X} bytes of data", length);
          chunk_data = Vec::with_capacity(length as usize);
          for i in 0..length {
            chunk_data.push(JpgReader::next(bytes)?);
          }
          last_byte = *chunk_data.last().unwrap_or(&last_byte);
          reader_state = JpgReaderState::processChunk;
          println!("Register states: {:02X} {:02X}", last_byte, byte);
        }
        JpgReaderState::processChunk => {
          println!("Processing chunk of type {:02X}", chunk_type);
          match chunk_type {
            n @ 0xE0...0xEF => {
              println!("Found chunk of 0x{:02X}", n);
              if n == TAGS_CHUNK_TYPE {
                println!("Tags may be found here!");
                match std::str::from_utf8(&chunk_data) {
                  Ok(string) => match JpgReader::parse_tags(string) {
                    Ok(tags_found) => tags = tags_found,
                    Err(_) => {}
                  },
                  Err(_) => {
                    println!("This is not an XML chunk of data :(");
                  }
                }
              }
            }
            0xD9 => {
              println!("Finished parsing");
              break;
            }
            _ => {}
          };
          reader_state = JpgReaderState::watingChunkType;
          byte = JpgReader::next(bytes)?;
        }
      }
    }
    Ok(tags)
  }
}
impl JpgReader {
  fn parse_tags(xml: &str) -> Result<HashSet<String>, Error> {
    let xml = xml
      .split_whitespace()
      .map(|v: &str| v.to_string() + " ")
      .collect::<String>();
    println!("{}", xml);
    return Ok(HashSet::new());
  }
}
