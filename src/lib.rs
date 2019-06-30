mod png;
mod reader;
pub mod error;

use error::Error;
use reader::Reader;
use std::fs::File;
use std::io::Read;
use std::collections::HashSet;

pub fn read_tags(path: String) -> Result<HashSet<String>, Error> {
  let mut bytes = File::open(path)?.bytes();
  png::PngReader::read_tags(&mut bytes)
}
