use crate::error::Error;
use crate::reader::Reader;

use std::collections::HashSet;
use std::io::{Bytes, Read};
const SIGNATURE: &[u8] = &[0xFF, 0xD8];

pub struct JpgReader;

impl Reader for JpgReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error> {
    let mut tags: HashSet<String> = HashSet::new();
    for byte in SIGNATURE.iter() {
      if *byte != JpgReader::next(bytes)? {
        return Err(Error::UnknownFormat);
      }
    }

    loop {
      let byte = match JpgReader::next(bytes) {
        Ok(byte)  => byte,
        Err(e) => return Err(Error::UnknownFormat),
      };
      if 0xFF == byte {
        let chunk_type = JpgReader::next(bytes)?;
        let mut chunk_size: u16;
        println!("Byte found: {}",chunk_type);
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
          0xDB => {
            println!("Found chunk of 0xDB");
            chunk_size = ((JpgReader::next(bytes)? as u16) << 8) | JpgReader::next(bytes)? as u16;
          }
          0xDD => {
            println!("Found chunk of 0xDD");
            chunk_size = 4;
          }
          n @ 0xD0...0xD7 => {
            println!("Found chunk of 0xDn");
            chunk_size = 0;
          }
          n @ 0xE0...0xEF => {
            println!("Found chunk of 0xEn");
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
            let bytes_around: Vec<u8> = bytes.take(255).take_while(|b|b.is_ok()).map(|b|b.unwrap()).collect();
            println!("Chunk type {} \nNext 255 bytes: {:02X?}", chunk_type, bytes_around);
            return Err(Error::UnknownFormat);
          }
        }
        println!("Skiping {}/{0:04X} bytes",chunk_size);
        while chunk_size>0 {
          JpgReader::next(bytes)?;
          chunk_size-=1;
        }
      }
    }

    Ok(tags)
  }
}