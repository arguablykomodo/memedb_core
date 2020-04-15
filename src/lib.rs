//! This crate provides functions to read and write a set of strings to various media file formats.
//!
//! It's been mainly designed for the categorization of memes.

type TagSet = std::collections::HashSet<String>;

/// Given a `src`, return the tags contained inside.
///
/// Pretty self explanatory, really.
pub fn read_tags(src: impl std::io::Read) -> TagSet {
    unimplemented!()
}

/// Write the provided `tags` to `dest`
///
/// Pretty self explanatory, really.
pub fn write_tags(dest: impl std::io::Write, tags: &TagSet) {
    unimplemented!()
}
