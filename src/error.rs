use custom_error::custom_error;

custom_error! { pub Error
    Format = "Invalid or unknown format",
    EOF = "Reached unexpected end of file",
    Io {source: std::io::Error} = "Error reading file"
}
