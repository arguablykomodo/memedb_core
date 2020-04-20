use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Tags are not valid UTF8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Corrupted tags in PNG data")]
    PngChecksum,
}

pub type Result<T> = std::result::Result<T, Error>;
