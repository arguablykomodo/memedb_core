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

use futures::AsyncReadExt;

use crate::utils::{or_eof, read_byte, read_heap};
use crate::utils::{read_byte_async, read_heap_async};
use std::io::Read;

/// One of the possible formats identified by [`identify_format`][crate::identify_format].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Format {
    /// [Graphics Interchange Format][crate::gif].
    #[cfg(feature = "gif")]
    Gif,
    /// [ISO Base Media File Format][crate::isobmff].
    #[cfg(feature = "isobmff")]
    Isobmff,
    /// [Joint Photographic Experts Group][crate::jpeg].
    #[cfg(feature = "jpeg")]
    Jpeg,
    /// [Portable Network Graphics][crate::png].
    #[cfg(feature = "png")]
    Png,
    /// [Resource Interchange File Format][crate::riff].
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

/// Attempts to identify the format of a given `src`.
///
/// The function operates based on a list of known "magic numbers" that can be found near the
/// beginning of most file formats.
///
/// If no known format can be identified, `None` will be returned.
pub async fn identify_format_async(
    src: &mut (impl AsyncReadExt + Unpin),
) -> Result<Option<Format>, std::io::Error> {
    let mut active = Vec::new();
    let mut next = FORMATS.to_vec();
    let mut i = 0;
    while let Some(byte) = or_eof(read_byte_async(src).await)? {
        let (new, still_next) = next.into_iter().partition(|f| f.offset == i);
        next = still_next;
        active = active.into_iter().chain(new).filter(|f| byte == f.magic[i - f.offset]).collect();
        i += 1;
        match active.len() {
            1 => {
                let FormatInfo { magic, offset, format } = active[0];
                let rest = read_heap_async(src, magic.len() + offset - i).await?;
                return Ok((rest == magic[i - offset..]).then_some(format));
            }
            0 if next.is_empty() => return Ok(None), // TODO: skip useless bytes
            _ => continue,
        }
    }
    Ok(None)
}

/// Attempts to identify the format of a given `src`.
///
/// The function operates based on a list of known "magic numbers" that can be found near the
/// beginning of most file formats.
///
/// If no known format can be identified, `None` will be returned.
pub fn identify_format(src: &mut impl Read) -> Result<Option<Format>, std::io::Error> {
    let mut active = Vec::new();
    let mut next = FORMATS.to_vec();
    let mut i = 0;
    while let Some(byte) = or_eof(read_byte(src))? {
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
    use futures::executor::block_on;

    #[test]
    fn correctly_identify_handlers_async() {
        for format in FORMATS {
            let mut bytes = vec![0; format.offset];
            bytes.extend_from_slice(format.magic);
            assert_eq!(
                block_on(identify_format_async(&mut &bytes[..])).unwrap(),
                Some(format.format)
            );
        }
    }

    #[test]
    fn correctly_identify_handlers() {
        for format in FORMATS {
            let mut bytes = vec![0; format.offset];
            bytes.extend_from_slice(format.magic);
            assert_eq!(identify_format(&mut &bytes[..]).unwrap(), Some(format.format));
        }
    }

    #[test]
    fn unknown_format_async() {
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(block_on(identify_format_async(&mut &bytes[..])).unwrap(), None);
    }

    #[test]
    fn unknown_format() {
        let bytes = &[0x2E, 0x7C, 0x2E, 0x2E, 0x0A, 0x2E, 0x2E, 0x7C, 0x2E, 0x2C];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }

    #[test]
    fn short_file_async() {
        let bytes = &[0x00];
        assert_eq!(block_on(identify_format_async(&mut &bytes[..])).unwrap(), None);
    }

    #[test]
    fn short_file() {
        let bytes = &[0x00];
        assert_eq!(identify_format(&mut &bytes[..]).unwrap(), None);
    }
}
