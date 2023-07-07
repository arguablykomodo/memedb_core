use thiserror::Error;

#[derive(Error, Debug)]
/// A possible error returned by a `read_tags` or `write_tags` function.
pub enum Error {
    /// The tags provided are invalid (check the documentation of
    /// [`is_tag_valid`][crate::is_tag_valid] for more information)
    #[error("Tags are not valid")]
    InvalidTags,
    /// There was an IO error while reading or writing the tags.
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    /// The tags being read do not constitute a valid UTF-8 string.
    #[error("Tags are not valid UTF-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// There is a mismatch between the calculated CRC-32 hash and the one found in the block.
    #[error("Corrupted tags in PNG data")]
    PngChecksum,
    /// The tags being written are larger than the maximum PNG/RIFF chunk size of 2^32-1 bytes.
    #[error("Size of tags overflows maximum chunk size")]
    ChunkSizeOverflow,
    /// An unknown GIF extension block was found. Possible extensions are:
    ///
    /// - Application extension (`0xFF`)
    /// - Comment extension (`0xFE`)
    /// - Graphics Control extension (`0xF9`)
    /// - Plaintext extension: (`0x01`)
    #[error("Unknown GIF extension block found, expected one of 0xFF, 0xFE, 0xF9, or 0x01, but found {0:X}")]
    GifUnknownExtension(u8),
    /// An unknown GIF block was found. Possible blocks are:
    ///
    /// - Extension block (`0x21`)
    /// - Image Descriptor block (`0x2C`)
    /// - End Of File block (`0x3B`)
    #[error("Unknown GIF block found, expected one of 0x21, 0x2C, or 0x3B, but found {0:X}")]
    GifUnknownBlock(u8),
    /// A GIF application extension identifier was with the wrong length was found. All application
    /// extension identifiers have to be 11 bytes wide.
    #[error("Application extension identifier should be 11, but found {0}")]
    GifWrongApplicationIdentifierLen(u8),
    /// Every JPEG segment must start with a 0xFF byte, this error is thrown if they don't.
    #[error("Segment Marker should be 0xFF, but found {0:X}")]
    JpegMissingSegmentMarker(u8),
    /// An unrecognized JPEG Segment was found.
    #[error("Unknown segment found, expected 0xC0-0xD7, 0xD9-0xEF, or 0xFE, but found {0:X}")]
    JpegUnknownSegment(u8),
    /// The header of the RIFF file declares a file length that is smaller than the sum of lengths
    /// reported by individual chunks.
    #[error("Chunk lengths conflict with length according to RIFF header")]
    InvalidRiffLength,
}
