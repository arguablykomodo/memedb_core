use std::io::{Read, Write};

pub fn read_byte(src: &mut impl Read) -> Result<u8, std::io::Error> {
    let mut byte = 0;
    src.read_exact(std::slice::from_mut(&mut byte))?;
    Ok(byte)
}

pub fn read_stack<const N: usize>(src: &mut impl Read) -> Result<[u8; N], std::io::Error> {
    let mut bytes = [0; N];
    src.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub fn read_heap(src: &mut impl Read, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = vec![0; n];
    src.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub fn skip(src: &mut impl std::io::Seek, n: i64) -> Result<u64, std::io::Error> {
    src.seek(std::io::SeekFrom::Current(n))
}

pub fn passthrough(
    src: &mut impl Read,
    dest: &mut impl Write,
    n: u64,
) -> Result<u64, std::io::Error> {
    std::io::copy(&mut src.take(n), dest)
}

pub fn or_eof<T>(x: Result<T, std::io::Error>) -> Result<Option<T>, std::io::Error> {
    use std::io::ErrorKind::UnexpectedEof;
    match x {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

macro_rules! standard_tests {
    ($e:literal) => {
        #[cfg(test)]
        mod standard_tests {
            use super::{read_tags, write_tags};
            use crate::are_tags_valid;
            use quickcheck_macros::quickcheck;
            use std::io::{BufRead, Cursor, Read, Seek};

            const UNTAGGED: &[u8] = include_bytes!(concat!("../../tests/media/minimal.", $e));
            const EMPTY: &[u8] = include_bytes!(concat!("../../tests/media/minimal_empty.", $e));
            const TAGGED: &[u8] = include_bytes!(concat!("../../tests/media/minimal_tagged.", $e));
            const LARGE: &[u8] = include_bytes!(concat!("../../tests/media/large.", $e));

            fn write(
                src: &mut (impl Read + BufRead + Seek),
                tags: impl IntoIterator<Item = impl AsRef<str>>,
            ) -> Vec<u8> {
                let mut buf = Vec::new();
                write_tags(src, &mut buf, tags).unwrap();
                buf
            }

            #[test]
            fn untagged() {
                assert_eq!(read_tags(&mut Cursor::new(&UNTAGGED)).unwrap(), &[] as &[&str]);
                assert_eq!(write(&mut Cursor::new(&UNTAGGED), &["bar", "foo"]), TAGGED);
            }

            #[test]
            fn empty() {
                assert_eq!(read_tags(&mut Cursor::new(&EMPTY)).unwrap(), &[] as &[&str]);
                assert_eq!(write(&mut Cursor::new(&EMPTY), &["bar", "foo"]), TAGGED);
            }

            #[test]
            fn tagged() {
                assert_eq!(read_tags(&mut Cursor::new(&TAGGED)).unwrap(), &["bar", "foo"]);
                assert_eq!(write(&mut Cursor::new(&TAGGED), &[] as &[&str]), EMPTY);
            }

            #[test]
            fn large() {
                assert_eq!(read_tags(&mut Cursor::new(&LARGE)).unwrap(), &[] as &[&str]);
            }

            #[quickcheck]
            fn qc_read_never_panics(bytes: Vec<u8>) -> bool {
                let _ = read_tags(&mut Cursor::new(&bytes));
                true
            }

            #[quickcheck]
            fn qc_write_never_panics(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                if are_tags_valid(&tags) {
                    let _ = write_tags(&mut Cursor::new(&bytes), &mut std::io::sink(), tags);
                }
                true
            }

            #[quickcheck]
            fn qc_identity(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                if are_tags_valid(&tags) && read_tags(&mut Cursor::new(&bytes)).is_ok() {
                    let mut dest = Vec::new();
                    if write_tags(&mut Cursor::new(bytes), &mut dest, tags.clone()).is_ok() {
                        return read_tags(&mut Cursor::new(dest)).unwrap() == tags;
                    }
                }
                true
            }
        }
    };
}

pub(crate) use standard_tests;
