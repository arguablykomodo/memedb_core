pub mod error;
mod gif;
mod png;

mod reader;
mod xml;
use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;

pub type TagSet = HashSet<String>;

pub fn read_tags(path: String) -> Result<TagSet, Error> {
    let mut file = File::open(&path)?;
    match path.split(".").last().unwrap() {
        "png" => png::PngReader::read_tags(&mut file),
        "gif" => gif::GifReader::read_tags(&mut file),
        _ => Err(Error::UnknownFormat),
    }
}

pub fn write_tags(path: &String, tags: &TagSet) -> Result<(), Error> {
    let mut file = File::open(&path)?;
    let bytes = match path.split(".").last().unwrap() {
        "png" => png::PngReader::write_tags(&mut file, &tags)?,
        "gif" => gif::GifReader::write_tags(&mut file, &tags)?,
        _ => return Err(Error::UnknownFormat),
    };
    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}
