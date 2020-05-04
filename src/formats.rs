use crate::{error::Result, TagSet};
use std::io::{Read, Seek, Write};

macro_rules! generate_formats {
    {$($module:ident => $variant:ident),*} => {
        $(mod $module;)*

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum Format {
            $($variant,)*
        }

        const FORMATS: &[(&[u8], Format)] = &[
            $(($module::SIGNATURE, Format::$variant),)*
        ];

        pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
            if let Some(format) = identify_format(src)? {
                Ok(Some(match format {
                    $(Format::$variant => $module::read_tags(src)?,)*
                }))
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
                Ok(Some(match format {
                    $(Format::$variant => $module::write_tags(src, dest, tags)?,)*
                }))
            } else {
                Ok(None)
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/format_macro.rs"));

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
            1 => return Ok(Some(formats[0].1)),
            0 => return Ok(None),
            _ => (),
        }
    }
    unreachable!()
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
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }
}
