//! This crate provides functions to read and write a set of strings to various media file formats.
//!
//! It has been mainly designed for the categorization of memes.

#[cfg(not(any(feature = "png", feature = "gif", feature = "riff")))]
compile_error!("At least one format feature must be enabled for this crate to be usable.");

#[macro_use]
mod utils;

mod error;
mod formats;

pub use error::Error;
use error::Result;
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
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
    formats::read_tags(src)
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
) -> Result<Option<()>> {
    if are_tags_valid(&tags) {
        formats::write_tags(src, dest, tags)
    } else {
        Err(error::Error::InvalidTags)
    }
}
