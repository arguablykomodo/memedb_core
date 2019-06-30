use crate::error::Error;
use crate::reader::Reader;
use std::io::{Bytes, Read};
use std::collections::HashSet;

const SIGNATURE: &[u8] = &[0xFF,0xD8,0xFF];

pub struct JpgReader;

impl Reader for JpgReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>,Error> {
    let mut tags: HashSet<String> = HashSet::new();
    for byte in SIGNATURE.iter() {
      if *byte != JpgReader::next(bytes)? {
        return Err(Error::UnknownFormat);
      }
    }
    Ok(tags)
  }
}