//! This crate provides functions to read and write a set of strings to various media file formats.
//!
//! It's been mainly designed for the categorization of memes.

mod error;
mod formats;

use error::Result;
use std::io;

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
/// # use memedb_core::{read_tags, tagset};
/// # fn main() -> std::io::Result<()> {
/// let tags = read_tags(&mut File::open("foo.png")?);
/// assert_eq!(tags.unwrap(), Some(tagset!{"bar"}));
/// # Ok(())
/// # }
/// ```
/// In the case that the format is unrecognized, the function will return None.
pub fn read_tags(src: &mut (impl io::Read + io::Seek)) -> Result<Option<TagSet>> {
    formats::read_tags(src)
}

/// Write the provided `tags` to `dest`
/// ```no_run
/// # use std::{fs::File, collections::HashSet};
/// # use memedb_core::{write_tags, tagset};
/// # fn main() -> std::io::Result<()> {
/// write_tags(&mut File::create("bar.png")?, &tagset!{"foo"});
/// # Ok(())
/// # }
/// ```
/// Pretty self explanatory, really.
pub fn write_tags(dest: &mut impl io::Write, tags: &TagSet) {
    unimplemented!()
}
