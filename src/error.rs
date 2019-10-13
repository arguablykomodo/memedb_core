use custom_error::custom_error;

custom_error! { pub Error
    Format = "Unknown format",
    Parser = "Error parsing file",
    EOF = "Reached unexpected end of file",
    Io {source: std::io::Error} = "Error reading file"
}
