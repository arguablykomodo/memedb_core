#[derive(Debug)]
/// A possible error returned by a `read_tags` or `write_tags` function.
pub enum Error {
    /// There was an IO error while reading or writing the tags.
    Io(std::io::Error),
    /// The tags being read do not constitute a valid UTF-8 string.
    Utf8(std::string::FromUtf8Error),
    /// An unknown GIF block was found. Possible blocks are:
    ///
    /// - Extension block (`0x21`)
    /// - Image Descriptor block (`0x2C`)
    /// - End Of File block (`0x3B`)
    GifUnknownBlock(u8),
    /// An invalid JPEG marker was found. A marker can take any value except 0x00 and 0xFF.
    JpegInvalidMarker(u8),
    /// There is a mismatch between the calculated CRC-32 hash and the one found in the block.
    PngChecksum(u32, u32),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Utf8(e) => write!(f, "tags are not valid utf-8: {e}"),
            Error::GifUnknownBlock(b) => write!(f, "unknown gif block found: {b:02X}",),
            Error::JpegInvalidMarker(b) => write!(f, "invalid jpeg marker found: {b:02X}"),
            Error::PngChecksum(a, b) => write!(f, "corrupted tags in png data: {a:04X} != {b:04X}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            _ => None,
        }
    }
}
