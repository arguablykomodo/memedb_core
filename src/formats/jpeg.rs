//! # Joint Photographic Experts Group
//!
//! JPEG data is organized in segments. Segments start with a `0xFF` byte, and a second byte
//! indicating the type of the segment. Depending on this type three things can happen:
//!
//! 1. The segment is zero-sized, and ends right there.
//! 2. The segment is variable sized, in which case there will be a two byte big endian length
//!    indicator (including the length itself) followed by the segment data.
//! 3. The segment is entropy-coded, and must be slogged through byte by byte until a 0xFF is found
//!    that *isn't* followed by `0x00`, which marks the end of the segment.
//!
//! A JPEG file consists of a list of segments, with several constraints on their order, the
//! relevant for our case being:
//!
//! - The first segment must be `SOI`.
//! - On JFIF files, the second segment must be `APP0` with the id `JFIF`.
//! - On Exif files, the second segment must be `APP1` with the id `Exif`.
//! - The last segment must be `EOI`.
//!
//! MemeDB stores its tags in an `APP4` segment with the id `MemeDB`.
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

use crate::{
    utils::{or_eof, passthrough, read_byte, read_heap, read_stack, skip},
    Error, TagSet,
};
use std::io::{Read, Seek, Write};

const TAGS_ID: &[u8] = b"MemeDB\x00";
const JFIF_ID: &[u8] = b"JFIF\x00";
const EXIF_ID: &[u8] = b"Exif\x00\x00";

fn read_marker(src: &mut (impl Read + Seek)) -> Result<u8, Error> {
    let marker = read_byte(src)?;
    if marker == 0xFF {
        Ok(read_byte(src)?)
    } else {
        Err(Error::InvalidSource("missing segment marker"))
    }
}

fn skip_segment(src: &mut (impl Read + Seek)) -> Result<(), Error> {
    let length = u16::from_be_bytes(read_stack::<2>(src)?).saturating_sub(2);
    skip(src, length as i64)?;
    Ok(())
}

fn skip_ecs(src: &mut (impl Read + Seek)) -> Result<u8, Error> {
    loop {
        if read_byte(src)? == 0xFF {
            let byte = read_byte(src)?;
            if byte != 0x00 {
                return Ok(byte);
            }
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<TagSet, Error> {
    skip(src, MAGIC.len() as i64)?;
    let mut byte = read_marker(src)?;
    loop {
        match byte {
            0x00..=0xBF | 0xD8 | 0xF0..=0xFD | 0xFF => {
                return Err(Error::InvalidSource("unknown jpeg segment"))
            }
            // APP4
            0xE4 => {
                let length = u16::from_be_bytes(read_stack::<2>(src)?).saturating_sub(2) as usize;
                if length < TAGS_ID.len() {
                    skip(src, length as i64)?;
                    byte = read_marker(src)?;
                } else if read_heap(src, TAGS_ID.len())? != TAGS_ID {
                    skip(src, length.saturating_sub(TAGS_ID.len()) as i64)?;
                    byte = read_marker(src)?;
                } else {
                    let length = length.saturating_sub(TAGS_ID.len());
                    let mut tags = TagSet::new();
                    let mut tag_src = src.take(length as u64);
                    while let Some(n) = or_eof(read_byte(&mut tag_src))? {
                        let tag = read_heap(&mut tag_src, n as usize)?;
                        tags.insert(String::from_utf8(tag)?);
                    }
                    return Ok(tags);
                }
            }
            // SOF, DHT, DAC, DQT, DNL, DRI, DHP, EXP, COM, APP
            0xC0..=0xCF | 0xDB..=0xDF | 0xFE | 0xE0..=0xEF => {
                skip_segment(src)?;
                byte = read_marker(src)?;
            }
            // SOS
            0xDA => {
                skip_segment(src)?;
                byte = skip_ecs(src)?;
            }
            // RST
            0xD0..=0xD7 => {
                byte = skip_ecs(src)?;
            }
            // EOI
            0xD9 => return Ok(TagSet::new()),
        }
    }
}

fn write_segment(src: &mut (impl Read + Seek), dest: &mut impl Write) -> Result<(), Error> {
    let length_bytes = read_stack::<2>(src)?;
    dest.write_all(&length_bytes)?;
    passthrough(src, dest, u16::from_be_bytes(length_bytes).saturating_sub(2) as u64)?;
    Ok(())
}

fn write_ecs(src: &mut (impl Read + Seek), dest: &mut impl Write) -> Result<u8, Error> {
    loop {
        let byte = read_byte(src)?;
        if byte == 0xFF {
            let second_byte = read_byte(src)?;
            if second_byte != 0x00 {
                return Ok(second_byte);
            }
            dest.write_all(&[byte, second_byte])?;
        } else {
            dest.write_all(&[byte])?;
        }
    }
}

fn write_tags_segment(dest: &mut impl Write, tags: TagSet) -> Result<(), Error> {
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tags = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.append(&mut tag.into_bytes());
        acc
    });
    dest.write_all(&[0xFF, 0xE4])?;
    dest.write_all(&((2 + TAGS_ID.len() + tags.len()) as u16).to_be_bytes())?;
    dest.write_all(TAGS_ID)?;
    dest.write_all(&tags)?;
    Ok(())
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> Result<(), Error> {
    skip(src, MAGIC.len() as i64)?;
    dest.write_all(MAGIC)?;
    let mut tags = Some(tags);
    let mut byte = read_marker(src)?;
    loop {
        match byte {
            0x00..=0xBF | 0xD8 | 0xF0..=0xFD | 0xFF => {
                return Err(Error::InvalidSource("unknown jpeg segment"))
            }
            // APP0-APP1
            0xE0..=0xE1 => {
                let length_bytes = read_stack::<2>(src)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                dest.write_all(&[0xFF, byte])?;
                dest.write_all(&length_bytes)?;
                let id = match byte {
                    0xE0 => read_heap(src, JFIF_ID.len())?,
                    0xE1 => read_heap(src, EXIF_ID.len())?,
                    _ => unreachable!(),
                };
                dest.write_all(&id)?;
                passthrough(src, dest, length.saturating_sub(id.len() as u16) as u64)?;
                if byte == 0xE0 && id == JFIF_ID || byte == 0xE1 && id == EXIF_ID {
                    if let Some(tags) = tags.take() {
                        write_tags_segment(dest, tags)?;
                    }
                }
                byte = read_marker(src)?;
            }
            // APP4
            0xE4 => {
                let length_bytes = read_stack::<2>(src)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2);
                let id = read_stack::<{ TAGS_ID.len() }>(src)?;
                if id != TAGS_ID {
                    dest.write_all(&[0xFF, byte])?;
                    dest.write_all(&length_bytes)?;
                    passthrough(src, dest, length.saturating_sub(TAGS_ID.len() as u16) as u64)?;
                }
                skip(src, length.saturating_sub(TAGS_ID.len() as u16) as i64)?;
                byte = read_marker(src)?;
            }
            // SOF, DHT, DAC, DQT, DNL, DRI, DHP, EXP, COM, APP
            0xC0..=0xCF | 0xDB..=0xDF | 0xFE | 0xE0..=0xEF => {
                dest.write_all(&[0xFF, byte])?;
                write_segment(src, dest)?;
                byte = read_marker(src)?;
            }
            // SOS
            0xDA => {
                dest.write_all(&[0xFF, byte])?;
                write_segment(src, dest)?;
                byte = write_ecs(src, dest)?;
            }
            // RST
            0xD0..=0xD7 => {
                dest.write_all(&[0xFF, byte])?;
                byte = write_ecs(src, dest)?;
            }
            // EOI
            0xD9 => {
                if let Some(tags) = tags.take() {
                    write_tags_segment(dest, tags)?;
                }
                dest.write_all(&[0xFF, byte])?;
                return Ok(());
            }
        }
    }
}

crate::utils::standard_tests!("jpeg");
