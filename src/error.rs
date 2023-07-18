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
    #[error("tags are not valid UTF-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// An unknown GIF block was found. Possible blocks are:
    ///
    /// - Extension block (`0x21`)
    /// - Image Descriptor block (`0x2C`)
    /// - End Of File block (`0x3B`)
    #[error("unknown GIF block found, expected one of 0x21, 0x2C, or 0x3B, but found {0:02X}")]
    GifUnknownBlock(u8),
    /// An invalid JPEG marker was found.
    #[error("invalid JPEG marker found, excpected anything but 0x00 or 0xFF, but found {0:02X}")]
    JpegInvalidMarker(u8),
    /// There is a mismatch between the calculated CRC-32 hash and the one found in the block.
    #[error("corrupted tags in PNG data")]
    PngChecksum,
}
