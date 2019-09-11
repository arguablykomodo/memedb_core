pub mod error;
mod gif;
mod jpg;
mod png;
mod reader;
mod xml;

use error::Error;
use reader::Reader;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

#[macro_use]
extern crate log;

pub type TagSet = HashSet<String>;

pub fn read_tags(path: &Path) -> Result<TagSet, Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    match path.extension().and_then(OsStr::to_str) {
        Some("png") => png::PngReader::read_tags(&mut file),
        Some("gif") => gif::GifReader::read_tags(&mut file),
        Some("jpg") | Some("jpeg") => jpg::JpgReader::read_tags(&mut file),
        _ => Err(Error::UnknownFormat),
    }
}

pub fn write_tags(path: &Path, tags: &TagSet) -> Result<(), Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    let bytes = match path.extension().and_then(OsStr::to_str) {
        Some("png") => png::PngReader::write_tags(&mut file, &tags)?,
        Some("gif") => gif::GifReader::write_tags(&mut file, &tags)?,
        Some("jpg") | Some("jpeg") => jpg::JpgReader::write_tags(&mut file, &tags)?,
        _ => return Err(Error::UnknownFormat),
    };
    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}
