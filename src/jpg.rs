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
        Err(e) => return Err(e),
      };
      if 0xFF == byte {
        let chunk_type = JpgReader::next(bytes)?;
        let mut chunk_size: u16;
        let mut is_tags_chunk = false;
        println!("------------------------------------------");
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
            println!("Found chunk of 0x{:02X}",n);
            chunk_size = 0;
          }
          n @ 0xE0...0xEF => {
            println!("Found chunk of 0x{:02X}",n);
            if n==0xE1 {
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
            let bytes_around: Vec<u8> = bytes.take(255).take_while(|b|b.is_ok()).map(|b|b.unwrap()).collect();
            println!("Unkown chunk type {:02X} \nNext 255 bytes: {:02X?}", chunk_type, bytes_around);
            return Err(Error::WrongFormat);
          }
        }
        chunk_size = chunk_size - 2;
        println!("Skiping {}/{0:04X} bytes",chunk_size);
        let mut skipped_bytes = Vec::with_capacity(chunk_size as usize);
        while chunk_size>0 {
          let byte = JpgReader::next(bytes)?;
          skipped_bytes.push(byte);
          chunk_size-=1;
        }
        match std::str::from_utf8(&skipped_bytes) {
            Ok(v) if is_tags_chunk => {
              println!("This was read on an APPn marker: '{}'",&v.split_whitespace().map(|v: &str|{v.to_string()+" "}).collect::<String>());
            },
            Err(_) | Ok(_) => {println!("These bytes were skipped {:02X?}...",skipped_bytes.iter().take(10).collect::<Vec<&u8>>());}
          };
      } else {
        let bytes_around: Vec<u8> = bytes.take(255).take_while(|b|b.is_ok()).map(|b|b.unwrap()).collect();
        println!("Error: Expected FF read {:02X}\nNext 255 bytes: {:02X?}",byte,bytes_around);
        return Err(Error::WrongFormat);
      }
    }

    Ok(tags)
  }
}