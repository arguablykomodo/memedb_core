mod error;
mod png;
mod reader;
pub mod tags;

use error::Error;
use reader::Reader;
use std::fs::File;
use std::io::Read;
use tags::Tags;

pub fn read_tags(path: String) -> Result<Tags, Error> {
  let mut bytes = File::open(path)?.bytes();
  png::PngReader::read_tags(&mut bytes)
}
