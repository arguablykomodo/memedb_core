use crate::tags::Tags;

static SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub fn read_tags(bytes: &[u8]) -> Option<Tags> {
  let mut bytes = bytes.iter();

  for signature_byte in SIGNATURE {
    if bytes.next()? != signature_byte {
      return None;
    }
  }

  Some(Tags::new())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read() {
    assert_eq!(read_tags(include_bytes!("../tests/jpg.jpg")), None);
    assert_eq!(
      read_tags(include_bytes!("../tests/png.png")),
      Some(Tags::new())
    );
  }
}