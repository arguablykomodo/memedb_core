use crate::error::Error;
use crate::reader::Reader;
use crate::xml::{XmlTag, XmlTree};
use colored::*;
use std::collections::HashSet;
use std::io::{Bytes, Read};
use std::time::SystemTime;

const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
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
  fn read_tags(file: &mut impl Read) -> Result<HashSet<String>, Error> {
    let started = SystemTime::now();
    let mut tags: HashSet<String> = HashSet::new();
    let mut file_iterator: std::iter::Peekable<_> = file
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
    let mut chunk_header: [u8; 2];
    let mut chunk_length: usize;
    loop {
      let file_iterator_ref = &mut file_iterator;
      let peeked: Option<u8> = read!(file_iterator_ref;peek);
      if peeked == Some(0xFF) {
        chunk_header = read!(file_iterator_ref; 2);
        //println!("Chunk header: {:#02X?}", chunk_header);
        if chunk_header[1] == 0xD9 {
          println!("{}", "EOF".green());
          break;
        }
        let peeked: Option<u8> = read!(file_iterator_ref;peek);
        if peeked == Some(0xFF) {
          //println!("Peeked the start of another chunk");
          continue;
        } else {
          //println!("Peeked the data");
        }

        chunk_length = 0x0000;
        chunk_length |= (read!(file_iterator_ref)? as usize) << 8;
        chunk_length |= read!(file_iterator_ref)? as usize;
        chunk_length -= 2;

        //println!("Chunk length: {:#06X?}", chunk_length);
        if read!(file_iterator_ref;peek) == Some(0x68) {
          let chunk_data: Vec<u8> = file_iterator_ref
            .take(chunk_length)
            .map(|v| v.unwrap())
            .collect();
          let chunk_string = String::from_utf8(chunk_data)
            .ok()
            .unwrap_or(":(".to_string());
          //println!("{}", chunk_string);
          match JpgReader::parse_xml(&chunk_string) {
            Ok(t) => tags = t,
            Err(e) => eprintln!("Xml parser error {}", format!("{:?}", e).red()),
          }
        } else {
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
  fn write_tags(file: &mut impl Read, tags: &HashSet<String>) -> Result<Vec<u8>, Error> {
    unimplemented!("Sorry dude, I can't do that yet");
  }
}
impl JpgReader {
  fn parse_xml(xml: &str) -> Result<HashSet<String>, Error> {
    let tree = XmlTree::parse(xml.to_string())?;
    let finds = tree.find_elements(|e: &XmlTag| match e.attributes.get("rdf:about") {
      Some(v) => v == KEYWORDS_UUID,
      None => false,
    });
    let mut tags: HashSet<String> = HashSet::new();
    for i in &finds {
      tree.traverse_map(
        *i,
        |tag: &XmlTag, tags: &mut HashSet<String>| {
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