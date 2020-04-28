mod png;

use crate::{error::Result, TagSet};
use std::io::{Read, Seek, Write};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Format {
    Png,
}

const FORMATS: &[(&[u8], Format)] = &[(png::SIGNATURE, Format::Png)];

// Identifies the format for a file by succesively eliminating non-matching signatures until 1 remains.
fn identify_format(src: &mut impl Read) -> Result<Option<Format>> {
    let mut formats = FORMATS.to_vec();

    // Get length of longest signature, so we know when to stop iterating
    let length = FORMATS.iter().map(|(s, _)| s.len()).max().expect("no handlers found");
    let mut buffer = [0]; // THIS IS STUPID
    for i in 0..length {
        src.read_exact(&mut buffer)?;
        // Filter non-matching signatures
        formats = formats.into_iter().filter(|(s, _)| s[i] == buffer[0]).collect();
        match formats.len() {
            1 => return Ok(Some(formats[0].1)),
            0 => return Ok(None),
            _ => (),
        }
    }
    unreachable!()
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
    Ok(if let Some(format) = identify_format(src)? {
        Some(match format {
            Format::Png => png::read_tags(src)?,
        })
    } else {
        None
    })
}

pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<Option<()>> {
    Ok(if let Some(format) = identify_format(src)? {
        Some(match format {
            Format::Png => png::write_tags(src, dest, tags)?,
        })
    } else {
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_identify_handlers() {
        for format in FORMATS {
            assert_eq!(identify_format(&mut &format.0[..]).unwrap(), Some(format.1));
        }
    }

    #[test]
    fn unknown_format() {
        let bytes = &[0x2Eu8, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }
}
