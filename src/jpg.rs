use crate::error::Error;
use crate::reader::Reader;
use crate::xml::{XmlTag, XmlTree};
use crate::TagSet;
use colored::*;
use std::collections::HashSet;
use std::io::Read;
use std::iter::Peekable;
use std::time::SystemTime;

const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0x68;
const EOF_CHUNK_TYPE: u8 = 0xD9;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

#[allow(unused_macros)]
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
    let started = SystemTime::now();
    let mut tags: TagSet = HashSet::new();
    let mut file_iterator: Peekable<_> = file
      .bytes()
      /* .enumerate()
      .map(|(a, v)| {
        println!("Req: {:#06X}", a);
        v
      }) */
      .peekable();
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
        //println!("Chunk header: {:#02X?}", chunk_header);
        if chunk_type == 0x00 {
          //eprintln!("{}", "Skipping 0xFF inside chunk data".yellow());
          continue;
        } else if chunk_type == EOF_CHUNK_TYPE {
          //println!("{}", "EOF".green());
          break;
        }
        if read!(file_iterator_ref;peek) == Some(0xFF) {
          //println!("Peeked the start of another chunk");
          continue;
        }
        chunk_length = JpgReader::get_chunk_length(&mut file_iterator_ref)?;
        //println!("Chunk length: {:#06X?}", chunk_length);
        if read!(file_iterator_ref;peek) == Some(TAGS_CHUNK_TYPE) {
          let chunk_data = JpgReader::get_chunk_data(&mut file_iterator_ref, chunk_length)?;
          let chunk_string = String::from_utf8(chunk_data)
            .ok()
            .unwrap_or(":(".to_string());
          //println!("{}", chunk_string);
          match JpgReader::parse_xml(&chunk_string) {
            Ok(t) => tags = t,
            Err(e) => eprintln!("Xml parser error {}", format!("{:?}", e).red()),
          }
        } else {
          //println!("Skipping {} bytes", chunk_length);
          for _ in 0..chunk_length {
            read!(file_iterator_ref)?;
          }
        }
      } else {
        //println!("{}", format!("0xFF expected, got {:#02X?}", peeked).red());
        file_iterator_ref.next();
        //std::thread::sleep(std::time::Duration::from_secs(2));
      }
    }
    println!("Time elapsed: {:#?}", started.elapsed().unwrap());
    Ok(tags)
  }
  fn write_tags(file: &mut impl Read, tags: &TagSet) -> Result<Vec<u8>, Error> {
    unimplemented!("Sorry dude, I can't do that yet");
  }
}
impl JpgReader {
  fn get_chunk_length(
    file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
  ) -> Result<usize, Error> {
    let mut chunk_length = 0x0000;
    chunk_length |= (read!(file_iterator)? as usize) << 8;
    chunk_length |= read!(file_iterator)? as usize;
    chunk_length -= 2;
    return Ok(chunk_length);
  }
  fn get_chunk_data(
    file_iterator: &mut Peekable<impl Iterator<Item = Result<u8, std::io::Error>>>,
    chunk_length: usize,
  ) -> Result<Vec<u8>, Error> {
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
        Ok(v) => Err(Error::ParserError),
        Err(e) => Err(e),
      };
    } else {
      return Ok(chunk_data);
    }
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
                tags.insert(value.clone());
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
}
