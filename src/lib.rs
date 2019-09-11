pub mod error;
mod gif;
mod jpg;
mod png;
mod reader;
mod xml;

use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;

#[macro_use]
extern crate log;

pub type TagSet = HashSet<String>;

pub fn read_tags(path: String) -> Result<TagSet, Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    match path.split(".").last().unwrap() {
        "png" => png::PngReader::read_tags(&mut file),
        "gif" => gif::GifReader::read_tags(&mut file),
        "jpg" | "jpeg" => jpg::JpgReader::read_tags(&mut file),
        _ => Err(Error::UnknownFormat),
    }
}

pub fn write_tags(path: &String, tags: &TagSet) -> Result<(), Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    let bytes = match path.split(".").last().unwrap() {
        "png" => png::PngReader::write_tags(&mut file, &tags)?,
        "gif" => gif::GifReader::write_tags(&mut file, &tags)?,
        "jpg" | "jpeg" => jpg::JpgReader::write_tags(&mut file, &tags)?,
        _ => return Err(Error::UnknownFormat),
    };
    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}
