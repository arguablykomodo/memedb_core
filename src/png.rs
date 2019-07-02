use crate::error::Error;
use crate::reader::Reader;

use std::collections::HashSet;
use std::io::{Bytes, Read, Seek, SeekFrom, Write};
const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct PngReader {}

impl Reader for PngReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error> {
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

      let mut chunk_type: [u8; 4] = [0; 4];
      for i in 0..4 {
        chunk_type[i] = PngReader::next(bytes)?;
      }

      if chunk_type == *b"meMe" {
        let mut data = Vec::new();
        for _ in 0..length {
          data.push(PngReader::next(bytes)?);
        }

        let mut tags = HashSet::new();
        let mut text = String::new();
        for byte in data.iter() {
          if *byte == b';' {
            tags.insert(text);
            text = String::new();
          } else {
            text.push(*byte as char);
          }
        }
        return Ok(tags);
      }

      // All PNG files must end with an IEND chunk
      if chunk_type == *b"IEND" {
        return Ok(HashSet::new());
      }

      // Every chunk ends with a 4 byte long checksum
      for _ in 0..(length + 4) {
        PngReader::next(bytes)?;
      }
    }
  }

  fn write_tags(
    file: &mut (impl Write + Read + Seek),
    tags: &HashSet<String>,
  ) -> Result<(), Error> {
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    if bytes[0..SIGNATURE.len()] != *SIGNATURE {
      return Err(Error::UnknownFormat);
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
      chunk_length.push((tags.len() >> (3 - i) * 8) as u8);
    }

    let mut new_chunk = Vec::new();
    new_chunk.append(&mut chunk_length);
    new_chunk.append(&mut vec![b'm', b'e', b'M', b'e']);
    new_chunk.append(&mut tags);
    new_chunk.append(&mut vec![0, 0, 0, 0]); // Empty checksum for now (see issue #1)

    let mut i = SIGNATURE.len();
    loop {
      let length = bytes[i..i + 4]
        .iter()
        .enumerate()
        .fold(0, |acc, (i, b)| (acc + *b as u32) << (3 - i) * 8);
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

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&bytes)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::{File, OpenOptions};

  #[test]
  fn test_read_invalid() {
    let mut bytes = File::open("tests/jpg.jpg").unwrap().bytes();
    // mem::discriminant magic is used to compare enums without having to implement PartialEq
    assert_eq!(
      std::mem::discriminant(&PngReader::read_tags(&mut bytes).unwrap_err()),
      std::mem::discriminant(&Error::UnknownFormat)
    );
  }

  #[test]
  fn test_read_empty() {
    let mut bytes = File::open("tests/read_empty.png").unwrap().bytes();
    let tags = HashSet::new();
    assert_eq!(PngReader::read_tags(&mut bytes).unwrap(), tags);
  }

  #[test]
  fn test_read_untagged() {
    let mut bytes = File::open("tests/read_untagged.png").unwrap().bytes();
    let tags = HashSet::new();
    assert_eq!(PngReader::read_tags(&mut bytes).unwrap(), tags);
  }

  #[test]
  fn test_read_tagged() {
    let mut bytes = File::open("tests/read_tagged.png").unwrap().bytes();
    let mut tags = HashSet::new();
    tags.insert("foo".to_owned());
    tags.insert("bar".to_owned());
    assert_eq!(PngReader::read_tags(&mut bytes).unwrap(), tags);
  }

  #[test]
  fn test_write_invalid() {
    let mut file = File::open("tests/jpg.jpg").unwrap();
    let mut tags = HashSet::new();
    tags.insert("foo".to_owned());
    tags.insert("bar".to_owned());
    // mem::discriminant magic is used to compare enums without having to implement PartialEq
    assert_eq!(
      std::mem::discriminant(&PngReader::write_tags(&mut file, &tags).unwrap_err()),
      std::mem::discriminant(&Error::UnknownFormat)
    );
  }

  #[test]
  fn test_write_empty() {
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .open("tests/write_empty.png")
      .unwrap();
    let mut tags = HashSet::new();
    tags.insert("foo".to_owned());
    tags.insert("bar".to_owned());
    assert_eq!(PngReader::write_tags(&mut file, &tags).unwrap(), ());

    // Verify file was written correctly
    let mut result = File::open("tests/write_empty.png").unwrap();
    let mut result_bytes = Vec::new();
    result.read_to_end(&mut result_bytes).unwrap();

    let mut test = File::open("tests/read_tagged.png").unwrap();
    let mut test_bytes = Vec::new();
    test.read_to_end(&mut test_bytes).unwrap();

    assert_eq!(result_bytes, test_bytes);
  }

  #[test]
  fn test_write_untagged() {
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .open("tests/write_untagged.png")
      .unwrap();
    let mut tags = HashSet::new();
    tags.insert("foo".to_owned());
    tags.insert("bar".to_owned());
    assert_eq!(PngReader::write_tags(&mut file, &tags).unwrap(), ());

    // Verify file was written correctly
    let mut result = File::open("tests/write_untagged.png").unwrap();
    let mut result_bytes = Vec::new();
    result.read_to_end(&mut result_bytes).unwrap();

    let mut test = File::open("tests/read_tagged.png").unwrap();
    let mut test_bytes = Vec::new();
    test.read_to_end(&mut test_bytes).unwrap();

    assert_eq!(result_bytes, test_bytes);
  }

  #[test]
  fn test_write_tagged() {
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .open("tests/write_tagged.png")
      .unwrap();
    let tags = HashSet::new();
    assert_eq!(PngReader::write_tags(&mut file, &tags).unwrap(), ());

    // Verify file was written correctly
    let mut result = File::open("tests/write_tagged.png").unwrap();
    let mut result_bytes = Vec::new();
    result.read_to_end(&mut result_bytes).unwrap();

    let mut test = File::open("tests/read_untagged.png").unwrap();
    let mut test_bytes = Vec::new();
    test.read_to_end(&mut test_bytes).unwrap();

    assert_eq!(result_bytes, test_bytes);
  }
}
