#[macro_use]
mod helpers;
#[macro_use]
mod reader_tests;

mod error;
mod gif;
mod jpg;
mod png;
mod reader;
mod xml;

pub use error::Error;
use log::info;
use reader::Reader;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Bytes, Read, Write};
use std::path::Path;

type TagSet = HashSet<String>;

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

// Function exorcized by SrKomodo
fn identify_file_type(bytes: &mut Bytes<impl BufRead>) -> Result<FileType, Error> {
    info!("Identifying file type");
    let mut readers = READERS.to_vec();
    let mut i = 0;
    loop {
        let byte = next!(bytes);
        readers = readers
            .iter()
            .filter(|(signature, _)| signature[i] == byte)
            .cloned() // Maybe there's a better way to do this
            .collect();
        i += 1;
        match readers.len() {
            1 => {
                let (signature, reader) = readers.get(0).unwrap();
                for j in i..signature.len() {
                    if signature[j] != next!(bytes) {
                        return Err(Error::Format);
                    }
                }
                info!("Going to use reader {:?}", reader);
                return Ok(*reader);
            }
            0 => return Err(Error::Format),
            _ => (),
        };
    }
}

/// Reads the file at `path` and returns the tags inside it if found
///
/// # Example
/// ```
/// # use std::path::Path;
/// # use std::collections::HashSet;
/// # use std::fs::{copy, remove_file};
/// # use memedb_core::{read_tags, Error};
/// #
/// # fn main() -> Result<(), Error> {
/// # copy("tests/png/tagged.png", "tagged_meme.png");
/// let mut tags = HashSet::new();
/// tags.insert("foo".to_string());
/// tags.insert("bar".to_string());
///
/// let read_tags = read_tags(&Path::new("tagged_meme.png"))?;
/// assert_eq!(tags, read_tags);
/// # remove_file("tagged_meme.png");
/// # Ok(())
/// # }
/// ```
pub fn read_tags(path: &Path) -> Result<TagSet, Error> {
    let file = File::open(&path)?;
    let mut bytes = BufReader::new(file).bytes();

    match identify_file_type(&mut bytes)? {
        FileType::png => png::PngReader::read_tags(&mut bytes),
        FileType::jpg => jpg::JpgReader::read_tags(&mut bytes),
        FileType::gif => gif::GifReader::read_tags(&mut bytes),
    }
}

/// Writes `tags` to the file at `path`
///
/// # Example
/// ```
/// # use std::path::Path;
/// # use std::collections::HashSet;
/// # use std::fs::{copy, remove_file};
/// # use memedb_core::{write_tags, Error};
/// #
/// # fn main() -> Result<(), Error> {
/// # copy("tests/png/untagged.png", "untagged_meme.png");
/// let mut tags = HashSet::new();
/// tags.insert("foo".to_string());
/// tags.insert("bar".to_string());
///
/// write_tags(&Path::new("untagged_meme.png"), &tags)?;
/// # remove_file("untagged_meme.png");
/// # Ok(())
/// # }
/// ```
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
    use std::fs::copy;
    use std::path::PathBuf;

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
        let mut tmp = PathBuf::from("tmp");
        std::fs::create_dir(&tmp).unwrap();
        tmp.push("file"); // So that .set_file_name doesn't overwrite the directory name
        for path in glob("tests/**/empty.*").unwrap().map(|f| f.unwrap()) {
            tmp.set_file_name(path.file_name().unwrap());
            println!("{:?} {:?}", path, tmp);
            copy(path, &tmp).unwrap();
            write_tags(&tmp, &tagset! {"foo", "bar"}).unwrap();
            assert_eq!(read_tags(&tmp).unwrap(), tagset! {"foo", "bar"});
        }
        for path in glob("tests/**/untagged.*").unwrap().map(|f| f.unwrap()) {
            tmp.set_file_name(path.file_name().unwrap());
            println!("{:?} {:?}", path, tmp);
            copy(path, &tmp).unwrap();
            write_tags(&tmp, &tagset! {"foo", "bar"}).unwrap();
            assert_eq!(read_tags(&tmp).unwrap(), tagset! {"foo", "bar"});
        }
        for path in glob("tests/**/tagged.*").unwrap().map(|f| f.unwrap()) {
            tmp.set_file_name(path.file_name().unwrap());
            println!("{:?} {:?}", path, tmp);
            copy(path, &tmp).unwrap();
            write_tags(&tmp, &tagset! {}).unwrap();
            assert_eq!(read_tags(&tmp).unwrap(), tagset! {});
        }
        tmp.pop();
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
