use crate::error::Error;
use crate::tags::Tags;
use std::io::{Bytes, Read};

pub trait Reader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<Tags, Error>;
  fn next(bytes: &mut Bytes<impl Read>) -> Result<u8, Error> {
    match bytes.next() {
      Some(b) => Ok(b?),
      None => Err(Error::UnexpectedEOF),
    }
  }
}
