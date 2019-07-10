pub mod error;
mod gif;
mod png;
mod reader;

use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;

pub type TagSet = HashSet<String>;

pub fn read_tags(path: String) -> Result<TagSet, Error> {
    let mut file = File::open(path)?;
    png::PngReader::read_tags(&mut file)
}

pub fn write_tags(path: &String, tags: &TagSet) -> Result<(), Error> {
    let mut file = File::open(&path)?;
    let bytes = png::PngReader::write_tags(&mut file, &tags)?;
    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}
