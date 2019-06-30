use crate::error::Error;
use std::io::{Bytes, Read};
use std::collections::HashSet;

pub trait Reader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error>;
  fn next(bytes: &mut Bytes<impl Read>) -> Result<u8, Error> {
    match bytes.next() {
      Some(b) => Ok(b?),
      None => Err(Error::UnexpectedEOF),
    }
  }
}
