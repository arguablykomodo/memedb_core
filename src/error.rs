#[derive(Debug)]
pub enum Error {
    UnknownFormat,
    UnexpectedEOF,
    IOError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}
