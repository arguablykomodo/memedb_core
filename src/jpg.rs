use crate::error::Error;
use crate::log::{debug, error, info, warn};
use crate::reader::Reader;
use crate::xml::{XmlTag, XmlTree};
use crate::TagSet;
use colored::*;
use std::collections::HashSet;
use std::io::Read;
use std::iter::Peekable;

const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
const EOF_CHUNK_TYPE: u8 = 0xD9;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

/* #region Debugging tools */
#[cfg(logAddresses)]
mod log_address {
  use std::iter::*;

  pub trait LogAddress<Item, I: Iterator<Item = Item>> {
    fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item>;
  }

  impl<Item, I> LogAddress<Item, I> for I
  where
    I: Iterator<Item = Item>,
  {
    fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item> {
      self.enumerate().map(&|(a, v)| {
        debug!("Address: 0x{:06X?}", a);
        v
      })
    }
  }
}
#[cfg(not(logAddresses))]
mod log_address {
  pub trait LogAddress<I: Iterator> {
    fn log<'a>(self) -> I;
  }
  impl<I> LogAddress<I> for I
  where
    I: Iterator,
  {
    fn log<'a>(self) -> I {
      self
    }
  }
}
/* #endregion */

macro_rules! read {
  ($i:ident) => {
    match $i.next() {
      Some(r) => match r {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::IOError(e)),
      },
      None => Err(Error::UnexpectedEOF),
    }
  };
  ($i:ident; $c: literal) => {{
    let mut out: [u8; $c] = [0xFF; $c];
    for (i, v) in $i.take($c).enumerate() {
      out[i] = v?;
    }
    out
  }};
  ($i:ident; peek) => {
    match $i.peek() {
      Some(r) => match r {
        Ok(v) => Some(*v),
        Err(_) => None,
      },
      None => None,
    }
  };
}

pub struct JpgReader;
impl Reader for JpgReader {
  fn read_tags(file: &mut impl Read) -> Result<TagSet, Error> {
    let mut tags: TagSet = HashSet::new();
    use log_address::LogAddress;
    let mut file_iterator: Peekable<_> = file.bytes().log().peekable();
    for byte in SIGNATURE.iter() {
      if *byte != read!(file_iterator)? {
        return Err(Error::UnknownFormat);
      }
    }
    let mut chunk_type: u8;
    let mut chunk_length: usize;
    loop {
      // Loops in rust have a bug where they consume variables in spite of borrowing them
      // making them unusable in the next iteration (thus failing even to compile)
      // Just declaring a dumb var and pointing it to the desired variable makes it usable in all the iterations
      let mut file_iterator_ref = &mut file_iterator;
      let peeked = match read!(file_iterator_ref) {
        Ok(v) => v,
        Err(_) => break,
      };
      if peeked == 0xFF {
        chunk_type = read!(file_iterator_ref)?;
        if read!(file_iterator_ref;peek) == Some(0xFF) {
          info!("Peeked the start of another chunk");
          continue;
        }
        info!("Chunk type: {:#02X?}", chunk_type);
        if chunk_type == 0x00 {
          //eprintln!("{}", "Skipping 0xFF inside chunk data".yellow());
          continue;
        } else if chunk_type == EOF_CHUNK_TYPE {
          info!("{}", "EOF".green());
          break;
        } else if chunk_type == TAGS_CHUNK_TYPE {
          let chunk_data = JpgReader::get_chunk_data(&mut file_iterator_ref)?;
          let chunk_string;

          if chunk_data[0] != 0x68 {
            continue;
          }

          chunk_string = match String::from_utf8(chunk_data) {
            Ok(v) => v,
            Err(e) => {
              error!(
                "Chunk data wasn't an XML (Failed with error {:#?})",
                format!("{:?}", e).red()
              );
              continue;
            }
          };
          info!("This is the XML found: '{}'", chunk_string.yellow());
          match JpgReader::parse_xml(&chunk_string) {
            Ok(t) => {
              tags = t;
              break;
            }
            Err(e) => eprintln!("XML parser error {}", format!("{:?}", e).red()),
          }
        } else {
          JpgReader::skip_chunk_data(&mut file_iterator_ref)?;
        }
      } else {
        file_iterator_ref.next();
        error!("Skipping bytes");
      }
    }
    Ok(tags)
  }
  fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error> {
    use std::time::SystemTime;
    let t = SystemTime::now();
    use log_address::LogAddress;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes);
    if bytes.len() == 0 || bytes[bytes.len() - 1] != 0xD9 {
      return Err(Error::ParserError);
    }
    let mut tags_address: usize = bytes.len() - 2;
    let mut windows = bytes.windows(2);
    for (addr, slice) in windows.enumerate() {
      if slice[0] != 0xFF {
        continue;
      }
      if slice[1] == TAGS_CHUNK_TYPE {
        info!("0xFFE1 found");
        tags_address = addr;
        break;
      }
    }

    // Look, NOBODY cares about the damn chunk length, so I will just leave it as 0x0000, k?
    let mut tags_bytes: Vec<u8> = vec![0xFF, TAGS_CHUNK_TYPE, 0x00, 0x00];
    for tag in tags {
      let tag: String = tag.to_string();
      tags_bytes.append(&mut tag.into_bytes());
    }
    for (i, b) in tags_bytes.iter().enumerate() {
      bytes.insert(tags_address + i, *b);
    }

    println!("{:#?}", t.elapsed());
    println!("Data recorded size: {}", bytes.len());
    return Ok(bytes);
  }
}
impl JpgReader {
  fn get_chunk_data(
    mut file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
  ) -> Result<Vec<u8>, Error> {
    let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
    let chunk_data: Vec<u8> = file_iterator
      .take(chunk_length)
      .map(|v| v.unwrap())
      .collect();
    if chunk_data.len() != chunk_length {
      eprintln!(
        "{}",
        format!(
          "Error: The data captured is shorter than expected\n{} bytes expected, got {}",
          chunk_length,
          chunk_data.len()
        )
        .red()
      );
      return match read!(file_iterator) {
        Ok(_) => Err(Error::ParserError),
        Err(e) => Err(e),
      };
    } else {
      return Ok(chunk_data);
    }
  }
  fn skip_chunk_data(
    mut file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
  ) -> Result<(), Error> {
    let chunk_length: usize = JpgReader::get_chunk_length(&mut file_iterator)?;
    for _ in 0..chunk_length {
      match file_iterator.next() {
        Some(v) => v?,
        None => return Err(Error::UnexpectedEOF),
      };
    }
    Ok(())
  }
  fn get_chunk_length(
    file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
  ) -> Result<usize, Error> {
    let mut chunk_length = 0x0000;
    chunk_length |= (read!(file_iterator)? as usize) << 8;
    chunk_length |= read!(file_iterator)? as usize;
    chunk_length -= 2;
    return Ok(chunk_length);
  }
  fn parse_xml(xml: &str) -> Result<TagSet, Error> {
    let tree = XmlTree::parse(xml.to_string())?;
    let finds = tree.find_elements(|e: &XmlTag| match e.attributes.get("rdf:about") {
      Some(v) => v == KEYWORDS_UUID,
      None => false,
    });
    let mut tags: TagSet = HashSet::new();
    for i in &finds {
      tree.traverse_map(
        *i,
        |tag: &XmlTag, tags: &mut TagSet| {
          if tag.name == "rdf:li" {
            match &tag.value {
              Some(value) => {
                tags.insert(value.to_string());
              }
              None => {}
            };
          }
          tags
        },
        &mut tags,
      );
    }
    Ok(tags)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::File;

  #[test]
  fn test_read_invalid() {
    let mut file = File::open("tests/invalid").unwrap();
    assert_eq!(
      std::mem::discriminant(&JpgReader::read_tags(&mut file).unwrap_err()),
      std::mem::discriminant(&Error::UnknownFormat)
    );
  }

  #[test]
  fn test_read_empty() {
    let mut file = File::open("tests/empty.jpeg").unwrap();
    let tags: Result<TagSet, Error> = JpgReader::read_tags(&mut file);
    assert!(tags.is_ok());
    let tags: TagSet = tags.unwrap();
    assert!(tags.is_empty());
  }

  #[test]
  fn test_read_tagged() {
    let mut file = File::open("tests/tagged.jpg").unwrap();
    let tags: Result<TagSet, Error> = JpgReader::read_tags(&mut file);
    assert!(tags.is_ok());
    let tags: TagSet = tags.unwrap();
    assert!(tags.contains("pepe"));
    assert!(tags.contains("fefe"));
  }

  #[test]
  fn test_write_empty() {
    let mut file = File::open("tests/empty.jpeg").unwrap();
    let mut tags = TagSet::new();
    tags.insert("pepe".to_string());
    let bytes = JpgReader::write_tags(&mut file, &tags);
  }
  #[test]
  fn test_write_tagged() {}
}
