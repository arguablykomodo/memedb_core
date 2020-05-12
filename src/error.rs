use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Tags are not valid UTF8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Corrupted tags in PNG data")]
    PngChecksum,
    #[error("Size of tags overflows maximum chunk size")]
    PngOverflow,
    #[error("Unknown GIF extension block found, expected one of 0xFF, 0xFE, 0xF9, or 0x01, but found {0:X}")]
    GifUnknownExtension(u8),
    #[error("Unknown GIF block found, expected one of 0x21, 0x2C, or 0x3B, but found {0:X}")]
    GifUnknownBlock(u8),
    #[error("Application extension identifier should be 11, but found {0}")]
    GifWrongApplicationIdentifierLen(u8),
}

pub type Result<T> = std::result::Result<T, Error>;
