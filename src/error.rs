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
    #[error("unknown GIF block found, expected one of 0x21, 0x2C, or 0x3B, but found {0:X}")]
    GifUnknownBlock(u8),

    /// The size field of the box being read marks it as smaller than its header, which is
    /// impossible.
    #[error("box size is smaller than possible")]
    IsobmffBoxTooSmall,
    /// The size field of the box being read marks it as larger than the size of the file, which is
    /// impossible.
    #[error("box size is larger than expected")]
    IsobmffBoxTooBig,

    /// Every JPEG segment must start with a 0xFF byte, this error is thrown if they don't.
    #[error("segment marker should be 0xFF, but found {0:X}")]
    JpegMissingSegmentMarker(u8),
    /// An unrecognized JPEG Segment was found.
    #[error("unknown segment found, expected 0xC0-0xD7, 0xD9-0xEF, or 0xFE, but found {0:X}")]
    JpegUnknownSegment(u8),

    /// There is a mismatch between the calculated CRC-32 hash and the one found in the block.
    #[error("corrupted tags in PNG data")]
    PngChecksum,
    /// The tags being written are larger than the maximum PNG chunk size of 2^32-1 bytes.
    #[error("size of tags overflows maximum chunk size")]
    PngChunkSizeOverflow,

    /// The header of the RIFF file declares a file length that is smaller than the sum of lengths
    /// reported by individual chunks.
    #[error("chunk lengths conflict with length according to RIFF header")]
    InvalidRiffLength,
    /// The tags being written are larger than the maximum RIFF chunk size of 2^32-1 bytes.
    #[error("size of tags overflows maximum chunk size")]
    RiffChunkSizeOverflow,
}
