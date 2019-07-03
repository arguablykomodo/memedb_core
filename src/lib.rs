pub mod error;
mod png;
mod reader;

use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

pub fn read_tags(path: String) -> Result<HashSet<String>, Error> {
  let mut bytes = File::open(path)?.bytes();
  png::PngReader::read_tags(&mut bytes)
}

pub fn write_tags(path: &String, tags: &HashSet<String>) -> Result<(), Error> {
  let mut file = File::open(&path)?;
  let bytes = png::PngReader::write_tags(&mut file, &tags)?;
  let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
  file.write_all(&bytes)?;
  Ok(())
}
