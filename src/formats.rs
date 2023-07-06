#[cfg(feature = "gif")]
mod gif;
#[cfg(feature = "isobmff")]
mod isobmff;
#[cfg(feature = "jpeg")]
mod jpeg;
#[cfg(feature = "png")]
mod png;
#[cfg(feature = "riff")]
mod riff;

use crate::{error::Result, TagSet, utils::{read_byte, read_heap}};
use std::io::{Read, Seek, Write};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FormatTag {
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
pub struct Format {
    magic: &'static [u8],
    offset: usize,
    tag: FormatTag,
}

impl Format {
    const fn new(magic: &'static [u8], offset: usize, tag: FormatTag) -> Self {
        Self { magic, offset, tag }
    }
}

const FORMATS: &[Format] = &[
    #[cfg(feature = "gif")]
    Format::new(gif::MAGIC, gif::OFFSET, FormatTag::Gif),
    #[cfg(feature = "isobmff")]
    Format::new(isobmff::MAGIC, isobmff::OFFSET, FormatTag::Isobmff),
    #[cfg(feature = "jpeg")]
    Format::new(jpeg::MAGIC, jpeg::OFFSET, FormatTag::Jpeg),
    #[cfg(feature = "png")]
    Format::new(png::MAGIC, png::OFFSET, FormatTag::Png),
    #[cfg(feature = "riff")]
    Format::new(riff::MAGIC, riff::OFFSET, FormatTag::Riff),
];

// Identifies the format for a file by succesively eliminating non-matching signatures until 1 remains.
fn identify_format(src: &mut impl Read) -> Result<Option<FormatTag>> {
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
                let Format { magic, offset, tag } = active[0];
                let rest = read_heap(src, magic.len() + offset - i)?;
                return Ok((rest == magic[i - offset..]).then_some(tag));
            }
            0 if next.is_empty() => return Ok(None), // TODO: skip useless bytes
            _ => continue,
        }
    }
    Ok(None)
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
    if let Some(format) = identify_format(src)? {
        src.seek(std::io::SeekFrom::Start(0))?;
        let tags = match format {
            #[cfg(feature = "gif")]
            FormatTag::Gif => gif::read_tags(src)?,
            #[cfg(feature = "isobmff")]
            FormatTag::Isobmff => isobmff::read_tags(src)?,
            #[cfg(feature = "jpeg")]
            FormatTag::Jpeg => jpeg::read_tags(src)?,
            #[cfg(feature = "png")]
            FormatTag::Png => png::read_tags(src)?,
            #[cfg(feature = "riff")]
            FormatTag::Riff => riff::read_tags(src)?,
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
        src.seek(std::io::SeekFrom::Start(0))?;
        match format {
            #[cfg(feature = "gif")]
            FormatTag::Gif => gif::write_tags(src, dest, tags)?,
            #[cfg(feature = "isobmff")]
            FormatTag::Isobmff => isobmff::write_tags(src, dest, tags)?,
            #[cfg(feature = "jpeg")]
            FormatTag::Jpeg => jpeg::write_tags(src, dest, tags)?,
            #[cfg(feature = "png")]
            FormatTag::Png => png::write_tags(src, dest, tags)?,
            #[cfg(feature = "riff")]
            FormatTag::Riff => riff::write_tags(src, dest, tags)?,
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
            let mut bytes = vec![0; format.offset];
            bytes.extend_from_slice(format.magic);
            assert_eq!(identify_format(&mut &bytes[..]).unwrap(), Some(format.tag));
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
