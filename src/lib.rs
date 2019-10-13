#[macro_use]
mod helpers;
#[macro_use]
mod reader_tests;

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
use std::io::{BufRead, BufReader, Bytes, Read, Write};
use std::path::Path;

pub type TagSet = HashSet<String>;

macro_rules! file_types {
    ($($name:ident),+) => {
        #[derive(Copy, Clone)]
        #[allow(non_camel_case_types)]
        enum FileType {
            $($name),+
        }

        const READERS: &[(&[u8], FileType)] = &[
            $(($name::SIGNATURE, FileType::$name)),+
        ];
    };
}

file_types!(png, jpg, gif);

fn identify_file_type(bytes: &mut Bytes<impl BufRead>) -> Result<FileType, Error> {
    let mut readers = READERS.to_vec();
    loop {
        let mut sigs_to_remove = vec![];
        let byte = next!(bytes);
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
    let file = File::open(&path)?;
    let mut bytes = BufReader::new(file).bytes();

    match identify_file_type(&mut bytes)? {
        FileType::png => png::PngReader::read_tags(&mut bytes),
        FileType::jpg => jpg::JpgReader::read_tags(&mut bytes),
        FileType::gif => gif::GifReader::read_tags(&mut bytes),
    }
}

pub fn write_tags(path: &Path, tags: &TagSet) -> Result<(), Error> {
    let file = File::open(&path)?;
    let mut bytes = BufReader::new(file).bytes();

    let bytes = match identify_file_type(&mut bytes)? {
        FileType::png => png::PngReader::write_tags(&mut bytes, &tags)?,
        FileType::jpg => jpg::JpgReader::write_tags(&mut bytes, &tags)?,
        FileType::gif => gif::GifReader::write_tags(&mut bytes, &tags)?,
    };

    let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
    file.write_all(&bytes)?;

    Ok(())
}
