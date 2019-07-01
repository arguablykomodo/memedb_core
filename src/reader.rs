use crate::error::Error;

use std::collections::HashSet;
use std::io::{Bytes, Read, Seek, Write};
pub trait Reader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error>;
  fn write_tags(file: &mut (impl Write + Read + Seek), tags: &HashSet<String>)
    -> Result<(), Error>;
  fn next(bytes: &mut Bytes<impl Read>) -> Result<u8, Error> {
    match bytes.next() {
      Some(b) => Ok(b?),
      None => Err(Error::UnexpectedEOF),
    }
  }
}
