mod png;

use std::io::{Bytes, Read, Result};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Handler {
    Png,
}

const HANDLERS: &[(&[u8], Handler)] = &[(png::SIGNATURE, Handler::Png)];

// Identifies the format for a file by succesively eliminating non-matching signatures until 1 remains.
fn identify_format(bytes: &mut Bytes<impl Read>) -> Result<Option<Handler>> {
    let mut handlers = HANDLERS.to_vec();
    // Get length of longest signature, so we know when to stop iterating
    let length = HANDLERS.iter().map(|(s, _)| s.len()).max().expect("no handlers found");
    for (i, byte) in bytes.take(length).enumerate() {
        let byte = byte?;
        // Filter non-matching signatures
        handlers = handlers.into_iter().filter(|(s, _)| s[i] == byte).collect();
        match handlers.len() {
            1 => return Ok(Some(handlers[0].1)),
            0 => return Ok(None),
            _ => (),
        }
    }
    // If we get here either the file is empty or something went *very* wrong
    if bytes.next().is_none() {
        Ok(None)
    } else {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_identify_handlers() {
        for handler in HANDLERS {
            assert_eq!(identify_format(&mut handler.0.bytes()).unwrap(), Some(handler.1));
        }
    }

    #[test]
    fn unknown_format() {
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(&mut bytes.bytes()).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(&mut bytes.bytes()).unwrap(), None);
    }

    #[test]
    fn empty_file() {
        let bytes = &[];
        assert_eq!(identify_format(&mut bytes.bytes()).unwrap(), None);
    }
}
