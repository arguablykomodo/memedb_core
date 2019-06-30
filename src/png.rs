use crate::error::Error;
use crate::reader::Reader;
use crate::tags::Tags;
use std::io::{Bytes, Read};

const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct PngReader {}

impl Reader for PngReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<Tags, Error> {
    for byte in SIGNATURE.iter() {
      if *byte != PngReader::next(bytes)? {
        return Err(Error::UnknownFormat);
      }
    }

    loop {
      let mut length = 0;
      for i in 0..4 {
        length += (PngReader::next(bytes)? as usize) << (3 - i) * 8;
      }
      println!("{}", length);

      let mut chunk_type: [u8; 4] = [0; 4];
      for i in 0..4 {
        chunk_type[i] = PngReader::next(bytes)?;
      }

      if chunk_type == *b"meMe" {
        let mut data = Vec::new();
        for _ in 0..length {
          data.push(PngReader::next(bytes)?);
        }

        let mut tags = Tags::new();
        let mut text = String::new();
        for byte in data.iter() {
          if *byte == b';' {
            tags.add_tag(text);
            text = String::new();
          } else {
            text.push(*byte as char);
          }
        }
        return Ok(tags);
      }

      // All PNG files must end with an IEND chunk
      if chunk_type == *b"IEND" {
        return Ok(Tags::new());
      }

      // Every chunk ends with a 4 byte long checksum
      for _ in 0..(length + 4) {
        PngReader::next(bytes)?;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::File;

  #[test]
  fn test_read() {
    let mut file = File::open("tests/jpg.jpg").unwrap().bytes();
    // mem::discriminant magic is used to compare enums without having to implement PartialEq
    assert_eq!(
      std::mem::discriminant(&PngReader::read_tags(&mut file).unwrap_err()),
      std::mem::discriminant(&Error::UnknownFormat)
    );

    let tags = Tags::new();
    let mut file = File::open("tests/empty.png").unwrap().bytes();
    assert_eq!(PngReader::read_tags(&mut file).unwrap(), tags);

    let mut tags = Tags::new();
    tags.add_tag(String::from("test"));
    let mut file = File::open("tests/tagged.png").unwrap().bytes();
    assert_eq!(PngReader::read_tags(&mut file).unwrap(), tags);
  }
}
