#[macro_use]
mod helpers;
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
use std::io::{BufReader, Read, Write};
use std::path::Path;

#[macro_use]
extern crate log;

pub type TagSet = HashSet<String>;

macro_rules! file_types {
    ($($name:ident),+) => {
        #[derive(Copy, Clone)]
        enum FileType {
            $($name),+
        }

        const READERS: &[(&[u8], FileType)] = &[
            $(($name::SIGNATURE, FileType::$name)),+
        ];
    };
}

file_types!(png, jpg, gif);

fn identify_file_type(file: impl Read) -> Result<FileType, Error> {
    let mut readers = READERS.to_vec();
    let mut bytes = BufReader::new(file).bytes();
    loop {
        let mut sigs_to_remove = vec![];
        let byte = bytes.next().unwrap().unwrap();
        for (i, (signature, _)) in readers.iter().enumerate() {
            if signature[i] != byte {
                sigs_to_remove.push(i);
            }
        }
        for sig in sigs_to_remove {
            readers.remove(sig);
        }
        match readers.len() {
            1 => return Ok(readers.get(0).unwrap().1),
            0 => return Err(Error::Format),
            _ => (),
        };
    }
}

pub fn read_tags(path: &Path) -> Result<TagSet, Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    match path.extension().and_then(OsStr::to_str) {
        Some("png") => png::PngReader::read_tags(&mut file),
        Some("gif") => gif::GifReader::read_tags(&mut file),
        Some("jpg") | Some("jpeg") => jpg::JpgReader::read_tags(&mut file),
        _ => Err(Error::Format),
    }
}

pub fn write_tags(path: &Path, tags: &TagSet) -> Result<(), Error> {
    info!("Debugging enabled");
    let mut file = File::open(&path)?;
    let bytes = match path.extension().and_then(OsStr::to_str) {
        Some("png") => png::PngReader::write_tags(&mut file, &tags)?,
        Some("gif") => gif::GifReader::write_tags(&mut file, &tags)?,
        Some("jpg") | Some("jpeg") => jpg::JpgReader::write_tags(&mut file, &tags)?,
        _ => return Err(Error::Format),
    };
    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}
