use crate::tags::Tags;

const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub fn read_tags(bytes: &[u8]) -> Option<Tags> {
  let mut i = 0;
  for signature_byte in SIGNATURE {
    if bytes[i] != *signature_byte {
      return None;
    }
    i += 1;
  }

  loop {
    let mut length = 0;
    for j in 0..4 {
      length += (bytes[i] as usize) << (3 - j) * 8;
      i += 1;
    }

    let chunk_type = &bytes[i..(i + 4)];
    i += 4;

    if chunk_type == b"meMe" {
      let data = &bytes[i..(i + length)];
      let mut tags = Tags::new();
      let mut text = String::new();
      for byte in data {
        if *byte == b';' {
          tags.add_tag(text);
          text = String::new();
        } else {
          text.push(*byte as char);
        }
      }
      return Some(tags);
    }

    // All PNG files must end with an IEND chunk
    if chunk_type == b"IEND" {
      return Some(Tags::new());
    }

    // Every chunk ends with a 4 byte long checksum
    i += length + 4;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read() {
    assert_eq!(read_tags(include_bytes!("../tests/jpg.jpg")), None);

    let tags = Tags::new();
    assert_eq!(read_tags(include_bytes!("../tests/empty.png")), Some(tags));

    let mut tags = Tags::new();
    tags.add_tag(String::from("test"));
    assert_eq!(read_tags(include_bytes!("../tests/tagged.png")), Some(tags));
  }
}
