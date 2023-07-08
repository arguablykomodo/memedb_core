use thiserror::Error;

#[derive(Error, Debug)]
/// A possible error returned by a `read_tags` or `write_tags` function.
pub enum Error {
    /// The tags provided are invalid (check the documentation of
    /// [`is_tag_valid`][crate::is_tag_valid] for more information)
    #[error("tags are not valid")]
    InvalidTags,
    /// There was an IO error while reading or writing the tags.
    #[error("io error")]
    Io(#[from] std::io::Error),
    /// The tags being read do not constitute a valid UTF-8 string.
    #[error("tags are not valid utf-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// There are errors in the source data that prevent the library from properly parsing the
    /// format. This could be due to corrupted data or incorrect spec compliance.
    #[error("error in source data: {0}")]
    InvalidSource(&'static str),
    /// The tags being written are larger than the maximum PNG/RIFF chunk size of 2^32-1 bytes.
    #[error("size of tags overflows maximum chunk size")]
    ChunkSizeOverflow,
}
