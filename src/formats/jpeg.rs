//! # Joint Photographic Experts Group
//!
//! JPEG data is separated by markers. Markers start with a sequence of one or more `0xFF` bytes,
//! followed by a second byte that identifies the marker.
//!
//! 1. Some markers are followed by no further data.
//! 2. Some markers comprise marker segments, which are followed by a 2 byte length field followed
//!    by the segment data.
//! 3. Some markers are followed by entropy-coded data, which have to be slogged through byte by
//!    byte until a `0xFF` byte is found that *isn't* followed by `0x00`, which marks the beggining
//!    of another marker.
//!
//! There are some constraints on the order of the markers:
//!
//! - The first marker must be `0xD8`.
//! - On JFIF files, the second marker segment must be `0xE0` with the id `JFIF`.
//! - On Exif files, the second marker segment must be `0xE1` with the id `Exif`.
//! - The last marker must be `0xD9`.
//!
//! MemeDB stores its tags in a `0xE4` segment with the id `MemeDB`.
//!
//! ## Relevant Links
//!
//! - [Wikipedia article for JPEG](https://en.wikipedia.org/wiki/JPEG)
//! - [Wikipedia article for JFIF](https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format)
//! - [The JPEG specification](https://www.w3.org/Graphics/JPEG/itu-t81.pdf)
//! - [The JFIF specification](https://www.w3.org/Graphics/JPEG/jfif3.pdf)
//! - [A description of the Exif file format](https://www.media.mit.edu/pia/Research/deepview/exif.html)

pub(crate) const MAGIC: &[u8] = b"\xFF\xD8";
pub(crate) const OFFSET: usize = 0;

use futures::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{
    utils::{decode_tags, encode_tags, passthrough, read_byte, read_heap, read_stack, skip},
    utils::{
        decode_tags_async, encode_tags_async, passthrough_async, read_byte_async, read_heap_async,
        read_stack_async, skip_async,
    },
    Error,
};
use std::io::{BufRead, Read, Seek, Write};

const TAGS_ID: &[u8] = b"MemeDB\x00";

async fn passthrough_ecs_async(
    src: &mut (impl AsyncReadExt + AsyncBufRead + Unpin),
    dest: &mut (impl AsyncWriteExt + Unpin),
) -> Result<u8, Error> {
    loop {
        let buf = src.fill_buf().await?;
        let len = buf.len();
        if len == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        }
        if let Some(i) = memchr::memchr(0xFF, buf) {
            dest.write_all(&buf[0..i]).await?;
            src.consume_unpin(i + 1);
            let mut byte = read_byte_async(src).await?;
            if byte == 0x00 {
                dest.write_all(&[0xFF, byte]).await?;
            } else {
                loop {
                    match byte {
                        0xFF => byte = read_byte_async(src).await?,
                        byte => return Ok(byte),
                    }
                }
            }
        } else {
            dest.write_all(buf).await?;
            src.consume_unpin(len);
        }
    }
}

fn passthrough_ecs(src: &mut (impl Read + BufRead), dest: &mut impl Write) -> Result<u8, Error> {
    loop {
        let buf = src.fill_buf()?;
        let len = buf.len();
        if len == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        }
        if let Some(i) = memchr::memchr(0xFF, buf) {
            dest.write_all(&buf[0..i])?;
            src.consume(i + 1);
            let mut byte = read_byte(src)?;
            if byte == 0x00 {
                dest.write_all(&[0xFF, byte])?;
            } else {
                loop {
                    match byte {
                        0xFF => byte = read_byte(src)?,
                        byte => return Ok(byte),
                    }
                }
            }
        } else {
            dest.write_all(buf)?;
            src.consume(len);
        }
    }
}

async fn read_marker_async(src: &mut (impl AsyncReadExt + Unpin)) -> Result<u8, Error> {
    let byte = read_byte_async(src).await?;
    if byte != 0xFF {
        return Err(Error::JpegInvalidMarker(byte));
    }
    loop {
        match read_byte_async(src).await? {
            0xFF => continue,
            byte => return Ok(byte),
        }
    }
}

fn read_marker(src: &mut impl Read) -> Result<u8, Error> {
    let byte = read_byte(src)?;
    if byte != 0xFF {
        return Err(Error::JpegInvalidMarker(byte));
    }
    loop {
        match read_byte(src)? {
            0xFF => continue,
            byte => return Ok(byte),
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub async fn read_tags_async(
    src: &mut (impl AsyncReadExt + AsyncBufReadExt + AsyncSeekExt + Unpin),
) -> Result<Vec<String>, Error> {
    let mut marker = read_marker_async(src).await?;
    loop {
        match marker {
            0xE4 => {
                let length =
                    u16::from_be_bytes(read_stack_async::<2>(src).await?).saturating_sub(2);
                if length < TAGS_ID.len() as u16 {
                    skip_async(src, length as i64).await?;
                } else if read_heap_async(src, TAGS_ID.len()).await? != TAGS_ID {
                    skip_async(src, length.saturating_sub(TAGS_ID.len() as u16) as i64).await?;
                } else {
                    return decode_tags_async(src).await;
                }
            }
            0xD9 => return Ok(Vec::new()),

            0x00 => return Err(Error::JpegInvalidMarker(marker)),
            0x01 | 0xD0..=0xD9 => {}
            0x02..=0xCF | 0xDA..=0xFE => {
                let length =
                    u16::from_be_bytes(read_stack_async::<2>(src).await?).saturating_sub(2);
                skip_async(src, length as i64).await?;
            }
            0xFF => unreachable!(),
        }
        marker = match marker {
            0xD0..=0xD7 | 0xDA => passthrough_ecs_async(src, &mut futures::io::sink()).await?,
            _ => read_marker_async(src).await?,
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + BufRead + Seek)) -> Result<Vec<String>, Error> {
    let mut marker = read_marker(src)?;
    loop {
        match marker {
            0xE4 => {
                let length = u16::from_be_bytes(read_stack::<2>(src)?).saturating_sub(2);
                if length < TAGS_ID.len() as u16 {
                    skip(src, length as i64)?;
                } else if read_heap(src, TAGS_ID.len())? != TAGS_ID {
                    skip(src, length.saturating_sub(TAGS_ID.len() as u16) as i64)?;
                } else {
                    return decode_tags(src);
                }
            }
            0xD9 => return Ok(Vec::new()),

            0x00 => return Err(Error::JpegInvalidMarker(marker)),
            0x01 | 0xD0..=0xD9 => {}
            0x02..=0xCF | 0xDA..=0xFE => {
                let length = u16::from_be_bytes(read_stack::<2>(src)?).saturating_sub(2);
                skip(src, length as i64)?;
            }
            0xFF => unreachable!(),
        }
        marker = match marker {
            0xD0..=0xD7 | 0xDA => passthrough_ecs(src, &mut std::io::sink())?,
            _ => read_marker(src)?,
        }
    }
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub async fn write_tags_async(
    src: &mut (impl AsyncReadExt + AsyncBufReadExt + AsyncSeekExt + Unpin),
    dest: &mut (impl AsyncWriteExt + Unpin),
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    passthrough_async(src, dest, 2).await?; // Assume SOI marker
    let mut tags = Some(tags);
    let mut marker = read_marker_async(src).await?;
    loop {
        if !matches!(marker, 0xE0 | 0xE1) {
            if let Some(tags) = tags.take() {
                let mut tags_bytes = Vec::new();
                encode_tags_async(tags, std::pin::pin!(&mut tags_bytes)).await?;
                dest.write_all(&[0xFF, 0xE4]).await?;
                dest.write_all(&((2 + TAGS_ID.len() + tags_bytes.len()) as u16).to_be_bytes())
                    .await?;
                dest.write_all(TAGS_ID).await?;
                dest.write_all(&tags_bytes).await?;
            }
        }
        match marker {
            0xE4 => {
                let length_bytes = read_stack_async::<2>(src).await?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                if length < TAGS_ID.len() as u16 {
                    dest.write_all(&[0xFF, marker]).await?;
                    dest.write_all(&length_bytes).await?;
                    passthrough_async(src, dest, length as u64).await?;
                } else if read_heap_async(src, TAGS_ID.len()).await? != TAGS_ID {
                    dest.write_all(&[0xFF, marker]).await?;
                    dest.write_all(&length_bytes).await?;
                    passthrough_async(
                        src,
                        dest,
                        length.saturating_sub(TAGS_ID.len() as u16) as u64,
                    )
                    .await?;
                } else {
                    skip_async(src, length.saturating_sub(TAGS_ID.len() as u16) as i64).await?;
                }
            }
            0xD9 => {
                dest.write_all(&[0xFF, marker]).await?;
                return Ok(());
            }

            0x00 => return Err(Error::JpegInvalidMarker(marker)),
            0x01 | 0xD0..=0xD9 => dest.write_all(&[0xFF, marker]).await?,
            0x02..=0xCF | 0xDA..=0xFE => {
                let length_bytes = read_stack_async::<2>(src).await?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                dest.write_all(&[0xFF, marker]).await?;
                dest.write_all(&length_bytes).await?;
                passthrough_async(src, dest, length as u64).await?;
            }
            0xFF => unreachable!(),
        }
        marker = match marker {
            0xD0..=0xD7 | 0xDA => passthrough_ecs_async(src, dest).await?,
            _ => read_marker_async(src).await?,
        }
    }
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub fn write_tags(
    src: &mut (impl Read + BufRead + Seek),
    dest: &mut impl Write,
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    passthrough(src, dest, 2)?; // Assume SOI marker
    let mut tags = Some(tags);
    let mut marker = read_marker(src)?;
    loop {
        if !matches!(marker, 0xE0 | 0xE1) {
            if let Some(tags) = tags.take() {
                let mut tags_bytes = Vec::new();
                encode_tags(tags, &mut tags_bytes)?;
                dest.write_all(&[0xFF, 0xE4])?;
                dest.write_all(&((2 + TAGS_ID.len() + tags_bytes.len()) as u16).to_be_bytes())?;
                dest.write_all(TAGS_ID)?;
                dest.write_all(&tags_bytes)?;
            }
        }
        match marker {
            0xE4 => {
                let length_bytes = read_stack::<2>(src)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                if length < TAGS_ID.len() as u16 {
                    dest.write_all(&[0xFF, marker])?;
                    dest.write_all(&length_bytes)?;
                    passthrough(src, dest, length as u64)?;
                } else if read_heap(src, TAGS_ID.len())? != TAGS_ID {
                    dest.write_all(&[0xFF, marker])?;
                    dest.write_all(&length_bytes)?;
                    passthrough(src, dest, length.saturating_sub(TAGS_ID.len() as u16) as u64)?;
                } else {
                    skip(src, length.saturating_sub(TAGS_ID.len() as u16) as i64)?;
                }
            }
            0xD9 => {
                dest.write_all(&[0xFF, marker])?;
                return Ok(());
            }

            0x00 => return Err(Error::JpegInvalidMarker(marker)),
            0x01 | 0xD0..=0xD9 => dest.write_all(&[0xFF, marker])?,
            0x02..=0xCF | 0xDA..=0xFE => {
                let length_bytes = read_stack::<2>(src)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                dest.write_all(&[0xFF, marker])?;
                dest.write_all(&length_bytes)?;
                passthrough(src, dest, length as u64)?;
            }
            0xFF => unreachable!(),
        }
        marker = match marker {
            0xD0..=0xD7 | 0xDA => passthrough_ecs(src, dest)?,
            _ => read_marker(src)?,
        }
    }
}

crate::utils::standard_tests!("jpeg");
