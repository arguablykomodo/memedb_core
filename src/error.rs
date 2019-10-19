use custom_error::custom_error;

custom_error! {
    /// All library functions return a `Result` with this error type
    pub Error
        /// This error is returned when the library cannot identify the file type you are attempting to read or write
        Format = "Unknown format",
        /// This error is returned when the filetype-specific parser encountered an error parsing the file, this is usually caused by corrupted/invalid files or strange encodings
        Parser = "Error parsing file",
        /// This error is returned when the file ended unexpectedly, this can be due to an IO error or corrupted data
        EOF = "Reached unexpected end of file",
        /// This error is returned when the OS encounters a problem while trying to read the file
        Io {source: std::io::Error} = "Error reading file"
}
