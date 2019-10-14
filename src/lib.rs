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
        #[derive(Copy, Clone, Debug, PartialEq)]
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
    /*
    1) This function SOMEHOW modifies the global state of the program (even though there is no such thing), therefore, it only works the first time
    2) As it only consume the least amount of byte needed to identify the file type, sometimes (read: always) the readers receive a half consumed signature, when they actually expect the signature to be missing
    3) `if signature[i] != byte` compares the i-nth byte of the current signature
    4) `if signature[i] != byte` compares always the same byte with the last byte read
    5) sigs_to_remove may not be in order, so when readers.remove(sig) is run, it may reduce the length of readers, leaving indexes in sigs_to_remove that are bigger than reader's length
    */
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

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;

    macro_rules! file_type_tests {
        ($($name:ident),+) => {
            $(
                for path in glob(concat!("tests/**/*.", stringify!($name)))
                    .unwrap()
                    .map(|f| f.unwrap())
                {
                    let file = File::open(&path).unwrap();
                    let mut bytes = BufReader::new(file).bytes();
                    let file_type = identify_file_type(&mut bytes).unwrap();
                    assert_eq!(file_type, FileType::$name);
                }
            )+
        };
    }

    #[test]
    fn test_identify_file_type() {
        file_type_tests!(png, gif, jpg);
    }

    #[test]
    fn test_read_tags() {
        for path in glob("tests/**/empty.*").unwrap().map(|f| f.unwrap()) {
            assert_eq!(read_tags(&path).unwrap(), tagset! {});
        }
        for path in glob("tests/**/untagged.*").unwrap().map(|f| f.unwrap()) {
            assert_eq!(read_tags(&path).unwrap(), tagset! {});
        }
        for path in glob("tests/**/tagged.*").unwrap().map(|f| f.unwrap()) {
            assert_eq!(read_tags(&path).unwrap(), tagset! {"foo", "bar"});
        }
    }

    #[test]
    fn test_write_tags() {
        unimplemented!();
    }
}
