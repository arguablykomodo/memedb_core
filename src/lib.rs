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

type TagSet = std::collections::HashSet<String>;

/// Utility macro for quickly creating tagsets.
///
/// ```
/// # use memedb_core::tagset;
/// let tagset = tagset!{ "foo", "bar" };
/// ```
#[macro_export]
macro_rules! tagset {
    {} => { std::collections::HashSet::new() };
    {$($tag:expr),*} => {{
        let mut tagset = std::collections::HashSet::new();
        $(tagset.insert(String::from($tag));)*
        tagset
    }};
}

/// Checks if a tag is valid.
///
/// Current restrictions:
///
/// - The tag cannot be an empty string.
/// - The tag cannot have a size higher than 256 bytes.
///
/// When writing tags, the library will call this function automatically and return an error if
/// appropriate.
pub fn is_tag_valid(tag: impl AsRef<str>) -> bool {
    !(tag.as_ref().is_empty() || tag.as_ref().len() > 0xFF)
}

/// Checks if a set of tags are valid.
///
/// Take a look at the documentation of [`is_tag_valid`](crate::is_tag_valid) for more information.
pub fn are_tags_valid(tags: &TagSet) -> bool {
    tags.iter().all(is_tag_valid)
}

/// Given a `src`, return the tags (if any) contained inside.
///
/// This function operates by first calling [`identify_format`](crate::identify_format), and then
/// calling the corresponding `read_tags` function if successful.
pub fn read_tags(src: &mut (impl Read + BufRead + Seek)) -> Result<Option<TagSet>, Error> {
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
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<Option<()>, Error> {
    if are_tags_valid(&tags) {
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
    } else {
        Err(error::Error::InvalidTags)
    }
}
