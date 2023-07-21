//! # MemeDB
//!
//! A Rust library for reading and writing tags to media streams.
//!
//! The library exposes the general purpose [`read_tags`][crate::read_tags] and
//! [`write_tags`][crate::read_tags] functions, which try to heuristically detect the format of the
//! source. For more specific use cases, each module in the library exposes specific `read_tags`
//! and `write_tags` functions for each format.

#[cfg(not(any(
    feature = "gif",
    feature = "isobmff",
    feature = "jpeg",
    feature = "png",
    feature = "riff"
)))]
compile_error!("At least one format feature must be enabled for this crate to be usable.");

mod error;
mod formats;
mod utils;

pub use error::Error;
pub use formats::*;
use std::io::{BufRead, Read, Seek, Write};

/// Given a `src`, return the tags (if any) contained inside.
///
/// This function operates by first calling [`identify_format`](crate::identify_format), and then
/// calling the corresponding `read_tags` function if successful.
pub fn read_tags(src: &mut (impl Read + BufRead + Seek)) -> Result<Option<Vec<String>>, Error> {
    if let Some(format) = identify_format(src)? {
        src.seek(std::io::SeekFrom::Start(0))?;
        let tags = match format {
            #[cfg(feature = "gif")]
            Format::Gif => gif::read_tags(src)?,
            #[cfg(feature = "isobmff")]
            Format::Isobmff => isobmff::read_tags(src)?,
            #[cfg(feature = "jpeg")]
            Format::Jpeg => jpeg::read_tags(src)?,
            #[cfg(feature = "png")]
            Format::Png => png::read_tags(src)?,
            #[cfg(feature = "riff")]
            Format::Riff => riff::read_tags(src)?,
        };
        Ok(Some(tags))
    } else {
        Ok(None)
    }
}

/// Read data from `src`, set the provided `tags`, and write to `dest`
///
/// This function will remove any tags that previously existed in the source.
///
/// This function operates by first calling [`identify_format`](crate::identify_format), and then
/// calling the corresponding `write_tags` function if successful.
pub fn write_tags(
    src: &mut (impl Read + BufRead + Seek),
    dest: &mut impl Write,
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Option<()>, Error> {
    if let Some(format) = identify_format(src)? {
        src.seek(std::io::SeekFrom::Start(0))?;
        match format {
            #[cfg(feature = "gif")]
            Format::Gif => gif::write_tags(src, dest, tags)?,
            #[cfg(feature = "isobmff")]
            Format::Isobmff => isobmff::write_tags(src, dest, tags)?,
            #[cfg(feature = "jpeg")]
            Format::Jpeg => jpeg::write_tags(src, dest, tags)?,
            #[cfg(feature = "png")]
            Format::Png => png::write_tags(src, dest, tags)?,
            #[cfg(feature = "riff")]
            Format::Riff => riff::write_tags(src, dest, tags)?,
        };
        Ok(Some(()))
    } else {
        Ok(None)
    }
}
