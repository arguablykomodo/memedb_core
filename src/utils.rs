use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use std::{
    io::{Read, Seek, Write},
    pin::Pin,
};

pub async fn read_byte_async(src: &mut (impl AsyncReadExt + Unpin)) -> Result<u8, std::io::Error> {
    let mut byte = 0;
    src.read_exact(std::slice::from_mut(&mut byte)).await?;
    Ok(byte)
}

pub fn read_byte(src: &mut impl Read) -> Result<u8, std::io::Error> {
    let mut byte = 0;
    src.read_exact(std::slice::from_mut(&mut byte))?;
    Ok(byte)
}

pub async fn read_stack_async<const N: usize>(
    src: &mut (impl AsyncReadExt + Unpin),
) -> Result<[u8; N], std::io::Error> {
    let mut bytes = [0; N];
    src.read_exact(&mut bytes).await?;
    Ok(bytes)
}

pub fn read_stack<const N: usize>(src: &mut impl Read) -> Result<[u8; N], std::io::Error> {
    let mut bytes = [0; N];
    src.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub async fn read_heap_async(
    src: &mut (impl AsyncReadExt + Unpin),
    n: usize,
) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = vec![0; n];
    src.read_exact(&mut bytes).await?;
    Ok(bytes)
}

pub fn read_heap(src: &mut impl Read, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = vec![0; n];
    src.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub async fn skip_async(
    src: &mut (impl AsyncSeekExt + Unpin),
    n: i64,
) -> Result<u64, std::io::Error> {
    src.seek(std::io::SeekFrom::Current(n)).await
}

pub fn skip(src: &mut impl Seek, n: i64) -> Result<u64, std::io::Error> {
    src.seek(std::io::SeekFrom::Current(n))
}

pub async fn passthrough_async(
    src: &mut (impl AsyncReadExt + Unpin),
    dest: &mut (impl AsyncWriteExt + Unpin),
    n: u64,
) -> Result<u64, std::io::Error> {
    futures::io::copy(src.take(n), dest).await
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

pub async fn encode_tags_async(
    tags: impl IntoIterator<Item = impl AsRef<str>>,
    mut dest: Pin<&mut impl AsyncWriteExt>,
) -> Result<(), std::io::Error> {
    for tag in tags {
        let mut tag_bytes: &[u8] = tag.as_ref().as_bytes();
        while tag_bytes.len() > 0b01111111 {
            dest.write_all(&[0b01111111]).await?;
            dest.write_all(&tag_bytes[0..0b01111111]).await?;
            tag_bytes = &tag_bytes[0b01111111..];
        }
        dest.write_all(&[tag_bytes.len() as u8 | 0b10000000]).await?;
        dest.write_all(tag_bytes).await?;
    }
    dest.write_all(&[0b00000000]).await?;
    Ok(())
}

pub fn encode_tags(
    tags: impl IntoIterator<Item = impl AsRef<str>>,
    dest: &mut impl Write,
) -> Result<(), std::io::Error> {
    for tag in tags {
        let mut tag_bytes: &[u8] = tag.as_ref().as_bytes();
        while tag_bytes.len() > 0b01111111 {
            dest.write_all(&[0b01111111])?;
            dest.write_all(&tag_bytes[0..0b01111111])?;
            tag_bytes = &tag_bytes[0b01111111..];
        }
        dest.write_all(&[tag_bytes.len() as u8 | 0b10000000])?;
        dest.write_all(tag_bytes)?;
    }
    dest.write_all(&[0b00000000])?;
    Ok(())
}

pub async fn decode_tags_async(
    src: &mut (impl AsyncReadExt + Unpin),
) -> Result<Vec<String>, crate::Error> {
    let mut tags = Vec::new();
    let mut tag_bytes = Vec::new();
    loop {
        let byte = read_byte_async(src).await?;
        match byte {
            0b00000000 => return Ok(tags),
            0b00000001..=0b01111111 => {
                passthrough_async(src, &mut tag_bytes, byte as u64).await?;
                continue;
            }
            0b10000000..=0b11111111 => {
                passthrough_async(src, &mut tag_bytes, (byte & 0b01111111) as u64).await?;
                tags.push(String::from_utf8(tag_bytes)?);
                tag_bytes = Vec::new();
            }
        }
    }
}

pub fn decode_tags(src: &mut impl Read) -> Result<Vec<String>, crate::Error> {
    let mut tags = Vec::new();
    let mut tag_bytes = Vec::new();
    loop {
        let byte = read_byte(src)?;
        match byte {
            0b00000000 => return Ok(tags),
            0b00000001..=0b01111111 => {
                passthrough(src, &mut tag_bytes, byte as u64)?;
                continue;
            }
            0b10000000..=0b11111111 => {
                passthrough(src, &mut tag_bytes, (byte & 0b01111111) as u64)?;
                tags.push(String::from_utf8(tag_bytes)?);
                tag_bytes = Vec::new();
            }
        }
    }
}

macro_rules! standard_tests {
    ($e:literal) => {
        #[cfg(test)]
        mod standard_tests {
            use super::{read_tags, read_tags_async, write_tags, write_tags_async};
            use futures::{
                executor::block_on, io::Cursor as AsyncCursor, AsyncBufReadExt, AsyncReadExt,
                AsyncSeekExt,
            };
            use quickcheck_macros::quickcheck;
            use std::io::{BufRead, Cursor, Read, Seek};

            const UNTAGGED: &[u8] = include_bytes!(concat!("../../tests/media/minimal.", $e));
            const EMPTY: &[u8] = include_bytes!(concat!("../../tests/media/minimal_empty.", $e));
            const TAGGED: &[u8] = include_bytes!(concat!("../../tests/media/minimal_tagged.", $e));
            const LARGE: &[u8] = include_bytes!(concat!("../../tests/media/large.", $e));

            async fn write_async(
                src: &mut (impl AsyncReadExt + AsyncBufReadExt + AsyncSeekExt + Unpin),
                tags: impl IntoIterator<Item = impl AsRef<str>>,
            ) -> Vec<u8> {
                let mut buf = Vec::new();
                write_tags_async(src, &mut buf, tags).await.unwrap();
                buf
            }

            fn write(
                src: &mut (impl Read + BufRead + Seek),
                tags: impl IntoIterator<Item = impl AsRef<str>>,
            ) -> Vec<u8> {
                let mut buf = Vec::new();
                write_tags(src, &mut buf, tags).unwrap();
                buf
            }

            #[test]
            fn untagged_async() {
                block_on(async {
                    assert_eq!(
                        read_tags_async(&mut AsyncCursor::new(&UNTAGGED)).await.unwrap(),
                        &[] as &[&str]
                    );
                    assert_eq!(
                        write_async(&mut AsyncCursor::new(&UNTAGGED), &["bar", "foo"]).await,
                        TAGGED
                    );
                });
            }

            #[test]
            fn untagged() {
                assert_eq!(read_tags(&mut Cursor::new(&UNTAGGED)).unwrap(), &[] as &[&str]);
                assert_eq!(write(&mut Cursor::new(&UNTAGGED), &["bar", "foo"]), TAGGED);
            }

            #[test]
            fn empty_async() {
                block_on(async {
                    assert_eq!(
                        read_tags_async(&mut AsyncCursor::new(&EMPTY)).await.unwrap(),
                        &[] as &[&str]
                    );
                    assert_eq!(
                        write_async(&mut AsyncCursor::new(&EMPTY), &["bar", "foo"]).await,
                        TAGGED
                    );
                });
            }

            #[test]
            fn empty() {
                assert_eq!(read_tags(&mut Cursor::new(&EMPTY)).unwrap(), &[] as &[&str]);
                assert_eq!(write(&mut Cursor::new(&EMPTY), &["bar", "foo"]), TAGGED);
            }

            #[test]
            fn tagged_async() {
                block_on(async {
                    assert_eq!(
                        read_tags_async(&mut AsyncCursor::new(&TAGGED)).await.unwrap(),
                        &["bar", "foo"]
                    );
                    assert_eq!(
                        write_async(&mut AsyncCursor::new(&TAGGED), &[] as &[&str]).await,
                        EMPTY
                    );
                });
            }

            #[test]
            fn tagged() {
                assert_eq!(read_tags(&mut Cursor::new(&TAGGED)).unwrap(), &["bar", "foo"]);
                assert_eq!(write(&mut Cursor::new(&TAGGED), &[] as &[&str]), EMPTY);
            }

            #[test]
            fn large_async() {
                assert_eq!(
                    block_on(read_tags_async(&mut AsyncCursor::new(&LARGE))).unwrap(),
                    &[] as &[&str]
                );
            }

            #[test]
            fn large() {
                assert_eq!(read_tags(&mut Cursor::new(&LARGE)).unwrap(), &[] as &[&str]);
            }

            #[quickcheck]
            fn qc_read_never_panics_async(bytes: Vec<u8>) -> bool {
                let _ = block_on(read_tags_async(&mut AsyncCursor::new(&bytes)));
                true
            }

            #[quickcheck]
            fn qc_read_never_panics(bytes: Vec<u8>) -> bool {
                let _ = read_tags(&mut Cursor::new(&bytes));
                true
            }

            #[quickcheck]
            fn qc_write_never_panics_async(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                let _ = block_on(write_tags_async(
                    &mut AsyncCursor::new(&bytes),
                    &mut futures::io::sink(),
                    tags,
                ));
                true
            }

            #[quickcheck]
            fn qc_write_never_panics(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                let _ = write_tags(&mut Cursor::new(&bytes), &mut std::io::sink(), tags);
                true
            }

            #[quickcheck]
            fn qc_identity_async(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                block_on(async {
                    if read_tags_async(&mut AsyncCursor::new(&bytes)).await.is_ok() {
                        let mut dest = Vec::new();
                        if write_tags_async(&mut AsyncCursor::new(bytes), &mut dest, tags.clone())
                            .await
                            .is_ok()
                        {
                            return read_tags_async(&mut AsyncCursor::new(dest)).await.unwrap()
                                == tags;
                        }
                    }
                    true
                })
            }

            #[quickcheck]
            fn qc_identity(bytes: Vec<u8>, tags: Vec<String>) -> bool {
                if read_tags(&mut Cursor::new(&bytes)).is_ok() {
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
