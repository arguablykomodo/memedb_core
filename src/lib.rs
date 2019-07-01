pub mod error;
mod png;
mod reader;

use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Read;

pub fn read_tags(path: String) -> Result<HashSet<String>, Error> {
  let mut bytes = File::open(path)?.bytes();
  png::PngReader::read_tags(&mut bytes)
}

pub fn write_tags(path: String, tags: HashSet<String>) -> Result<(), Error> {
  let mut file = OpenOptions::new()
    .read(true)
    .write(true)
    .open(path)
    .unwrap();
  png::PngReader::write_tags(&mut file, &tags)?;
  Ok(())
}
