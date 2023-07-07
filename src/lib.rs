//! This crate provides functions to read and write a set of strings to various media file formats.
//!
//! It has been mainly designed for the categorization of memes.

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
use std::io::{Read, Seek, Write};

type TagSet = std::collections::HashSet<String>;

/// Utility macro for quickly creating tagsets.
/// ```
/// # use std::collections::HashSet;
/// # use memedb_core::tagset;
/// // Creating tagsets the old-fashioned way is such a chore
/// let mut tagset_a = HashSet::new();
/// tagset_a.insert(String::from("foo"));
/// tagset_a.insert(String::from("bar"));
///
/// // But using the macro makes it a breeze!
/// let tagset_b = tagset!{ "foo", "bar" };
///
/// // And provides the same results!
/// assert_eq!(tagset_a, tagset_b);
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
/// appropiate.
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
/// ```no_run
/// # use std::fs::File;
/// # use memedb_core::{read_tags};
/// let tags = read_tags(&mut File::open("foo.png")?);
/// # Ok::<(), std::io::Error>(())
/// ```
/// In the case that the format is unrecognized, the function will return None.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>, Error> {
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
/// ```no_run
/// # use std::{fs::File};
/// # use memedb_core::{write_tags, tagset};
/// write_tags(&mut File::open("bar.png")?, &mut File::create("bar2.png")?, tagset! { "foo" });
/// # Ok::<(), std::io::Error>(())
/// ```
/// This function will remove any tags that previously existed in the source.
///
/// In the case that the format is unrecognized, the function will return None.
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
