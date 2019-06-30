use crate::error::Error;
use crate::reader::Reader;

use std::collections::HashSet;
use std::io::{Bytes, Read};
const SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];

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
      let may_byte: Result<_, _> = JpgReader::next(bytes);
      if may_byte.is_err() {
        break;
      }
      if 0xFF == may_byte.unwrap() {
        let chunk_type = JpgReader::next(bytes)?;
        let chunk_size;
        match chunk_type {
          0xD8 => {
            println!("Found chunk of 0xD8");
            chunk_size = 0;
          }
          0xC0 => {
            println!("Found chunk of 0xC0");
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
          }
          0xC2 => {
            println!("Found chunk of 0xC2");
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
          }
          0xC4 => {
            println!("Found chunk of 0xC4");
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
          }
          0xDB => {
            println!("Found chunk of 0xDB");
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
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
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
          }
          0xFE => {
            println!("Found chunk of 0xFE");
            chunk_size = JpgReader::next(bytes)? << 8 + JpgReader::next(bytes)?;
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
        for i in 0..chunk_size {
          JpgReader::next(bytes)?;
        }
      }
    }

    Ok(tags)
  }
}