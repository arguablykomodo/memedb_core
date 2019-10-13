// Not sure why these macros show as unused
// probably because they are only used inside another macro

#[allow(unused_macros)]
macro_rules! tagset {
    {$($tag:expr),+} => {{
        let mut m = TagSet::new();
        $(m.insert($tag.to_string());)*
        m
    }};
}

#[allow(unused_macros)]
macro_rules! open_file {
    ($path:expr, $n:expr) => {
        BufReader::new(File::open($path).unwrap()).bytes().skip($n)
    };
}

#[allow(unused_macros)]
macro_rules! file_path {
    ($extension:expr, $type:expr) => {
        concat!("tests/", $extension, "/", $type, ".", $extension)
    };
}

macro_rules! reader_tests {
    ($reader:ident, $extension:expr) => {
        #[cfg(test)]
        mod tests {
            use super::*;
            use std::fs::File;
            use std::io::{BufReader, Read};

            #[test]
            fn test_read_empty() {
                let mut file = open_file!(file_path!($extension, "empty"), SIGNATURE.len());
                let result = $reader::read_tags(&mut file).unwrap();
                assert_eq!(result, TagSet::new());
            }

            #[test]
            fn test_read_tagged() {
                let mut file = open_file!(file_path!($extension, "tagged"), SIGNATURE.len());
                let result = $reader::read_tags(&mut file).unwrap();
                assert_eq!(result, tagset! {"foo", "bar"});
            }

            #[test]
            fn test_write_empty() {
                let mut empty = open_file!(file_path!($extension, "empty"), SIGNATURE.len());
                let result = $reader::write_tags(&mut empty, &tagset! {"foo", "bar"}).unwrap();
                let tagged = open_file!(file_path!($extension, "tagged"), 0)
                    .map(|b| b.unwrap())
                    .collect::<Vec<u8>>();
                assert_eq!(result, tagged);
            }

            #[test]
            fn test_write_tagged() {
                let mut tagged = open_file!(file_path!($extension, "tagged"), SIGNATURE.len());
                let result = $reader::write_tags(&mut tagged, &TagSet::new()).unwrap();
                let empty = open_file!(file_path!($extension, "untagged"), 0)
                    .map(|b| b.unwrap())
                    .collect::<Vec<u8>>();
                assert_eq!(result, empty);
            }
        }
    };
}
