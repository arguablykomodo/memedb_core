//! This crate provides functions to read and write a set of strings to various media file formats.
//!
//! It's been mainly designed for the categorization of memes.

#[cfg(not(any(feature = "png", feature = "gif")))]
compile_error!("At least one format feature must be enabled for this crate to be usable.");

#[macro_use]
mod utils;

mod error;
mod formats;

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
/// let tagset_b = tagset!("foo", "bar");
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

/// Given a `src`, return the tags (if any) contained inside.
/// ```no_run
/// # use std::fs::File;
/// # use memedb_core::{read_tags};
/// # fn main() -> std::io::Result<()> {
/// let tags = read_tags(&mut File::open("foo.png")?);
/// # Ok(())
/// # }
/// ```
/// In the case that the format is unrecognized, the function will return None.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Option<TagSet>> {
    formats::read_tags(src)
}

/// Read data from `src`, add the provided `tags`, and write to `dest`
/// ```no_run
/// # use std::{fs::File};
/// # use memedb_core::{write_tags, tagset};
/// # fn main() -> std::io::Result<()> {
/// write_tags(&mut File::open("bar.png")?, &mut File::create("bar2.png")?, tagset! { "foo" });
/// # Ok(())
/// # }
/// ```
/// In the case that the format is unrecognized, the function will return None.
pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<Option<()>> {
    formats::write_tags(src, dest, tags)
}
