#[cfg(feature = "gif")]
pub mod gif;
#[cfg(feature = "isobmff")]
pub mod isobmff;
#[cfg(feature = "jpeg")]
pub mod jpeg;
#[cfg(feature = "png")]
pub mod png;
#[cfg(feature = "riff")]
pub mod riff;

use crate::{
    utils::{read_byte, read_heap},
    Error,
};
use std::io::Read;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Format {
    #[cfg(feature = "gif")]
    Gif,
    #[cfg(feature = "isobmff")]
    Isobmff,
    #[cfg(feature = "jpeg")]
    Jpeg,
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "riff")]
    Riff,
}

#[derive(Copy, Clone, Debug)]
struct FormatInfo {
    magic: &'static [u8],
    offset: usize,
    format: Format,
}

impl FormatInfo {
    const fn new(magic: &'static [u8], offset: usize, format: Format) -> Self {
        Self { magic, offset, format }
    }
}

const FORMATS: &[FormatInfo] = &[
    #[cfg(feature = "gif")]
    FormatInfo::new(gif::MAGIC, gif::OFFSET, Format::Gif),
    #[cfg(feature = "isobmff")]
    FormatInfo::new(isobmff::MAGIC, isobmff::OFFSET, Format::Isobmff),
    #[cfg(feature = "jpeg")]
    FormatInfo::new(jpeg::MAGIC, jpeg::OFFSET, Format::Jpeg),
    #[cfg(feature = "png")]
    FormatInfo::new(png::MAGIC, png::OFFSET, Format::Png),
    #[cfg(feature = "riff")]
    FormatInfo::new(riff::MAGIC, riff::OFFSET, Format::Riff),
];

// Identifies the format for a file by succesively eliminating non-matching signatures until 1 remains.
pub fn identify_format(src: &mut impl Read) -> Result<Option<Format>, Error> {
    let mut active = Vec::new();
    let mut next = FORMATS.to_vec();
    let mut i = 0;
    while let Some(byte) = read_byte(src).map_or_else(
        |e| if e.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(e) },
        |b| Ok(Some(b)),
    )? {
        let (new, still_next) = next.into_iter().partition(|f| f.offset == i);
        next = still_next;
        active = active.into_iter().chain(new).filter(|f| byte == f.magic[i - f.offset]).collect();
        i += 1;
        match active.len() {
            1 => {
                let FormatInfo { magic, offset, format } = active[0];
                let rest = read_heap(src, magic.len() + offset - i)?;
                return Ok((rest == magic[i - offset..]).then_some(format));
            }
            0 if next.is_empty() => return Ok(None), // TODO: skip useless bytes
            _ => continue,
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_identify_handlers() {
        for format in FORMATS {
            let mut bytes = vec![0; format.offset];
            bytes.extend_from_slice(format.magic);
            assert_eq!(identify_format(&mut &bytes[..]).unwrap(), Some(format.format));
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
