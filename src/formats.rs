#[cfg(feature = "gif")]
mod gif;
#[cfg(feature = "jpeg")]
mod jpeg;
#[cfg(feature = "png")]
mod png;
#[cfg(feature = "riff")]
mod riff;

use crate::{error::Result, TagSet};
use std::io::{Read, Seek, Write};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Format {
    #[cfg(feature = "gif")]
    Gif,
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "riff")]
    Riff,
    #[cfg(feature = "jpeg")]
    Jpeg,
}

const FORMATS: &[(&[u8], Format)] = &[
    #[cfg(feature = "gif")]
    (gif::SIGNATURE, Format::Gif),
    #[cfg(feature = "png")]
    (png::SIGNATURE, Format::Png),
    #[cfg(feature = "riff")]
    (riff::SIGNATURE, Format::Riff),
    #[cfg(feature = "jpeg")]
    (jpeg::SIGNATURE, Format::Jpeg),
];

// Identifies the format for a file by succesively eliminating non-matching signatures until 1 remains.
fn identify_format(src: &mut impl Read) -> Result<Option<Format>> {
    let mut formats = FORMATS.to_vec();

    // Get length of longest signature, so we know when to stop iterating
    let length = FORMATS.iter().map(|(s, _)| s.len()).max().expect("no handlers found");
    for i in 0..length {
        let byte = read_bytes!(src, 1);
        // Filter non-matching signatures
        formats = formats.into_iter().filter(|(s, _)| s[i] == byte).collect();
        match formats.len() {
            1 => {
                let format = formats[0];
                // Verify the rest of the signature
                if read_bytes!(src, format.0.len() - i - 1)[..] == format.0[i + 1..] {
                    return Ok(Some(format.1));
                }
            }
            0 => return Ok(None),
            _ => (),
        }
    }
    unreachable!()
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
    if let Some(format) = identify_format(src)? {
        let tags = match format {
            #[cfg(feature = "gif")]
            Format::Gif => gif::read_tags(src)?,
            #[cfg(feature = "png")]
            Format::Png => png::read_tags(src)?,
            #[cfg(feature = "riff")]
            Format::Riff => riff::read_tags(src)?,
            #[cfg(feature = "jpeg")]
            Format::Jpeg => jpeg::read_tags(src)?,
        };
        Ok(Some(tags))
    } else {
        Ok(None)
    }
}

pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<Option<()>> {
    if let Some(format) = identify_format(src)? {
        match format {
            #[cfg(feature = "gif")]
            Format::Gif => gif::write_tags(src, dest, tags)?,
            #[cfg(feature = "png")]
            Format::Png => png::write_tags(src, dest, tags)?,
            #[cfg(feature = "riff")]
            Format::Riff => riff::write_tags(src, dest, tags)?,
            #[cfg(feature = "jpeg")]
            Format::Jpeg => jpeg::write_tags(src, dest, tags)?,
        };
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_identify_handlers() {
        for format in FORMATS {
            assert_eq!(identify_format(&mut &*format.0).unwrap(), Some(format.1));
        }
    }

    #[test]
    fn unknown_format() {
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }
}
