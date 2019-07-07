use crate::error::Error;
use crate::reader::Reader;

use std::collections::HashSet;
use std::io::{Bytes, Read};

/* Explanation:
  1. JpgReader verifies the signature

  [loop]
  2. JpgReader tries to find a chunk
    a. If the value in the var "byte" is an 0xFF then
        that means the byte it has now is the chunk-type.
        Finally try to guess if this chunk has data by going to [3]
    b. if it is not then that probably means the
        reader is inside a data chunk. JpgReader discards
        the value, reads a new one and then goes to [2]

  3. JpgReader reads the next byte
    a. If it is an 0xFF, then this is the start of another chunk.
      Save this value in the var "byte", process this chunk by going to [4]
    b. Otherwise, it reads the next byte and appends them like this: (FIRST<<8 | SECOND)

  4. Finally process the chunk
    a. If the chunk type is 0xEn, then tags may be found here
    b. If the chunk type is 0xD9, then this is the end of the file. The break the loop and finish by doing [5]
    c. Any other type of chunk is skipped

  5. Finish and return the tags
*/
const SIGNATURE: &[u8] = &[0xFF, 0xD8];
const TAGS_CHUNK_TYPE: u8 = 0xE1;
const KEYWORDS_UUID: &str = "\"uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b\"";

enum JpgReaderState {
  WatingChunkType,
  RecordingChunkData(u16), //Data length
  WatingChunkLength,
  ProcessChunk,
}
pub struct JpgReader;
impl Reader for JpgReader {
  fn read_tags(bytes: &mut Bytes<impl Read>) -> Result<HashSet<String>, Error> {
    let mut tags: HashSet<String> = HashSet::new();
    for byte in SIGNATURE.iter() {
      if *byte != JpgReader::next(bytes)? {
        return Err(Error::UnknownFormat);
      }
    }

    let mut chunk_type = 0x00;
    let mut reader_state = JpgReaderState::WatingChunkType;
    let mut last_byte = *SIGNATURE.last().unwrap();
    let mut byte = JpgReader::next(bytes)?;
    let mut chunk_data: Vec<u8> = vec![];
    loop {
      match reader_state {
        JpgReaderState::WatingChunkType if last_byte == 0xFF => {
          println!("Retrieving chunk type");
          chunk_type = byte;
          println!("Chunk type: {:02X}", chunk_type);
          // If we are in the end of the file, we manually set it to finish the parsing
          reader_state = if chunk_type == 0xD9 {
            JpgReaderState::ProcessChunk
          } else {
            JpgReaderState::WatingChunkLength
          };
        }
        JpgReaderState::WatingChunkType => {
          println!("Waiting chunk type...");
          last_byte = byte;
          byte = JpgReader::next(bytes)?;
          println!("Read byte: {:02X}{:02X?}", last_byte, byte);
        }
        JpgReaderState::WatingChunkLength => {
          println!("Getting chunk length");
          let next_byte = JpgReader::next(bytes)?;
          if next_byte == 0xFF {
            println!("0-length chunk");
            reader_state = JpgReaderState::RecordingChunkData(0);
            byte = next_byte;
          } else {
            last_byte = next_byte;
            byte = JpgReader::next(bytes)?;
            let chunk_length = ((last_byte as u16) << 8) | (byte as u16);
            println!("Chunk length: {:04X}", chunk_length);
            reader_state = JpgReaderState::RecordingChunkData(chunk_length);
          }
        }
        JpgReaderState::RecordingChunkData(length) => {
          let length = if length > 0 { length - 2 } else { 0 };
          println!("Storing {:04X} bytes of data", length);
          chunk_data = Vec::with_capacity(length as usize);
          for i in 0..length {
            chunk_data.push(JpgReader::next(bytes)?);
          }
          last_byte = *chunk_data.last().unwrap_or(&last_byte);
          reader_state = JpgReaderState::ProcessChunk;
          println!("Register states: {:02X} {:02X}", last_byte, byte);
        }
        JpgReaderState::ProcessChunk => {
          println!("Processing chunk of type {:02X}", chunk_type);
          match chunk_type {
            0xE0...0xEF => {
              println!("Found chunk of 0x{:02X}", chunk_type);
              if chunk_type == TAGS_CHUNK_TYPE {
                println!("Tags may be found here!");
                match std::str::from_utf8(&chunk_data) {
                  Ok(string) => match JpgReader::parse_tags(string) {
                    Ok(tags_found) => tags = tags_found,
                    Err(_) => {}
                  },
                  Err(_) => {
                    println!("This is not an XML chunk of data :(");
                  }
                }
              }
            }
            0xD9 => {
              println!("Finished parsing");
              break;
            }
            _ => {}
          };
          reader_state = JpgReaderState::WatingChunkType;
          byte = JpgReader::next(bytes)?;
        }
      }
    }
    Ok(tags)
  }
}
impl JpgReader {
  fn parse_tags(xml: &str) -> Result<HashSet<String>, Error> {
    Xml::parse(xml.to_string());
    unimplemented!();
    /* let mut tags: HashSet<String> = HashSet::new();
    let xml: String = xml
      .split_whitespace()
      .map(|v: _| v.to_string() + " ")
      .map(|v: _| v.replace("<", "\n<").replace(">", ">\n"))
      .collect();
    let mut xml_stack: Vec<_> = Vec::new();
    let mut current_tag: Option<&str> = None;
    let mut inside_tag = false;
    for token in xml.split_whitespace() {
      let token: &str = token;
      println!("Token '{}'", token);
      if token.starts_with("<") {
        xml_stack.push(token.replace("<", ""));
        inside_tag = !token.ends_with(">");
      }
      if token.starts_with("</") || token.ends_with("/>") {
        xml_stack.pop();
        inside_tag = false;
        continue;
      }
      if inside_tag {
        let data: Vec<&str> = token.split("=").collect();
        let (key, value): (&&str, &&str) = (data.get(0).unwrap(), data.get(1).unwrap_or(&""));
        if *value == KEYWORDS_UUID {
          println!("Wiiiii!!! tags!");
        } else {
          println!("<{}> = <{}>", key, value);
        }
        continue;
      }
      if !inside_tag {
        let tag_stack_len = xml_stack.len();
        if xml_stack.len() > 3 && xml_stack[tag_stack_len - 1] == "rdf:Bag" {
          println!("This is a tag: {}", token);
          tags.insert(token.to_string());
        } else {
          println!("Value: {}", token);
        }
      }
    }
    return Ok(tags); */
  }
}

mod Xml {
  #[derive(PartialEq)]
  enum XmlParserState {
    WaitingTag,
    ReadingTagProps,
    ReadingTagValues,
  }
  #[derive(Debug)]
  struct XmlTag {
    name: String,
    props: Vec<String>,
    values: Vec<String>,
    children: Vec<XmlTag>,
  }
  pub fn parse(text: String) {
    let tokens: _ = text
      .replace("<", "\n<") // These 3 add whitespaces around the start and end of the tags so they can be easily split with the next function
      .replace(">", " >\n") // like this: <rdf::RDF> --> \n<rdf:RDF\s>\n
      .replace("/ >", " />") // transform /\s> into \s/>
      .split_ascii_whitespace()
      .skip_while(|v| *v != "<rdf:RDF") // Skip untl the begining of the file
      .map(|v: &str| v.to_string()) // Transform everything into Strings
      .collect::<Vec<String>>();
    println!("{:#?}", tokens);
    let mut tag_stack: Vec<XmlTag> = vec![];
    let mut processing_tag: Option<XmlTag> = None;
    let mut parser_state = XmlParserState::WaitingTag;

    let mut tokens_iter: std::iter::Peekable<_> = tokens.into_iter().peekable();
    loop {
      let peeked: &String = match tokens_iter.peek() {
        Some(v) => v,
        None => break,
      };
      if peeked.starts_with("<") {
        let tag = parse_tag(&mut tokens_iter);
        println!("Tag: {}", tag);
      } else {
        println!("OwO, whats dis: {}", peeked);
        tokens_iter.next();
      }
    }
    /*for token in tokens {
      let token: String = token;
      if token.starts_with("<") {
        if processing_tag.is_some() {
          tag_stack.push(processing_tag.unwrap());
          processing_tag = None;
        }
        processing_tag = Some(XmlTag {
          name: token
            .trim_matches(|c| c == '<' || c == '>' || c == '/')
            .to_string(),
          props: vec![],
          values: vec![],
          children: vec![],
        });

        if token.ends_with("/>") {
          parser_state = XmlParserState::WaitingTag;
          tag_stack.push(processing_tag.unwrap());
          processing_tag = None;
        } else if token.ends_with(">") || token.starts_with("<") {
          parser_state = XmlParserState::ReadingTagValues;
          tag_stack.push(processing_tag.unwrap());
          processing_tag = None;
        } else {
          parser_state = XmlParserState::ReadingTagProps;
        }
      }
      if token.ends_with(">") {
        if token.ends_with("/>") {
          parser_state = XmlParserState::WaitingTag;
        } else {
          parser_state = XmlParserState::ReadingTagValues;
        }
      }
      if !token.starts_with("<") && !token.ends_with(">") {
        if token == "pepe" {
          println!("lol");
        }
        if parser_state == XmlParserState::ReadingTagProps {
          match processing_tag {
            Some(ref mut v) => v.props.push(token),
            _ => {}
          }
        } else if parser_state == XmlParserState::ReadingTagValues {
          tag_stack.last_mut().unwrap().values.push(token);
        }
      }
    }*/
    println!("{:#?}", tag_stack);
  }
  fn parse_tag<T>(iter: T) -> String
  where
    T: Iterator<Item = String>,
  {
    let mut full_tag = String::from("");
    let mut last_token_read = String::from("");
    for token in iter.take_while(|v| {
      last_token_read = v.to_string();
      !v.ends_with(">")
    }) {
      full_tag += " ";
      full_tag += &token;
    }
    full_tag += &last_token_read;

    return full_tag;
  }
}
