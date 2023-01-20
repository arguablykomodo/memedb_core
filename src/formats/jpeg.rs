// JPEG files are made out of segments that start with 0xFF followed by a marker indicating what
// kind of segment it is. Variable length segments also have 2 more bytes indicating the length.
// Some segments are followed by entropy encoded data, that have to be read byte by byte until a
// 0xFF byte is found that *isn't* followed by 0x00.
//
// Related links:
// https://en.wikipedia.org/wiki/JPEG_File_Interchange_Format
// https://en.wikipedia.org/wiki/JPEG#Syntax_and_structure
// https://www.w3.org/Graphics/JPEG/itu-t81.pdf
// https://www.w3.org/Graphics/JPEG/jpeg3.pdf
// https://www.media.mit.edu/pia/Research/deepview/exif.html

use crate::{
    error::{Error, Result},
    TagSet,
};
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = b"\xFF\xD8";

const TAGS_ID: &[u8] = b"MemeDB\x00";
const JFIF_ID: &[u8] = b"JFIF\x00";
const EXIF_ID: &[u8] = b"Exif\x00\x00";

fn read_marker(src: &mut (impl Read + Seek)) -> Result<u8> {
    let marker = read_bytes!(src, 1)?;
    if marker == 0xFF {
        Ok(read_bytes!(src, 1)?)
    } else {
        Err(Error::JpegMissingSegmentMarker(marker))
    }
}

fn skip_segment(src: &mut (impl Read + Seek)) -> Result<()> {
    let length = u16::from_be_bytes(read_bytes!(src, 2)?).saturating_sub(2);
    skip_bytes!(src, length as i64)?;
    Ok(())
}

fn skip_ecs(src: &mut (impl Read + Seek)) -> Result<u8> {
    loop {
        if read_bytes!(src, 1)? == 0xFF {
            let byte = read_bytes!(src, 1)?;
            if byte != 0x00 {
                return Ok(byte);
            }
        }
    }
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    let mut byte = read_marker(src)?;
    loop {
        match byte {
            0x00..=0xBF | 0xD8 | 0xF0..=0xFD | 0xFF => return Err(Error::JpegUnknownSegment(byte)),
            // APP4
            0xE4 => {
                let length = u16::from_be_bytes(read_bytes!(src, 2)?).saturating_sub(2) as usize;
                if length < TAGS_ID.len() {
                    skip_bytes!(src, length as i64)?;
                    byte = read_marker(src)?;
                } else if read_bytes!(src, TAGS_ID.len() as u64)? != TAGS_ID {
                    skip_bytes!(src, length.saturating_sub(TAGS_ID.len()) as i64)?;
                    byte = read_marker(src)?;
                } else {
                    let length = length.saturating_sub(TAGS_ID.len());
                    let mut bytes = read_bytes!(src, length as u64)?;
                    let mut tags = TagSet::new();
                    while !bytes.is_empty() {
                        let size = bytes.remove(0) as usize;
                        let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                        tags.insert(String::from_utf8(bytes)?);
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
            0xD9 => return Ok(crate::TagSet::new()),
        }
    }
}

fn write_segment(src: &mut (impl Read + Seek), dest: &mut impl Write) -> Result<()> {
    let length_bytes = read_bytes!(src, 2)?;
    dest.write_all(&length_bytes)?;
    dest.write_all(&read_bytes!(src, u16::from_be_bytes(length_bytes).saturating_sub(2) as u64)?)?;
    Ok(())
}

fn write_ecs(src: &mut (impl Read + Seek), dest: &mut impl Write) -> Result<u8> {
    loop {
        let byte = read_bytes!(src, 1)?;
        if byte == 0xFF {
            let second_byte = read_bytes!(src, 1)?;
            if second_byte != 0x00 {
                return Ok(second_byte);
            }
            dest.write_all(&[byte, second_byte])?;
        } else {
            dest.write_all(&[byte])?;
        }
    }
}

fn write_tags_segment(dest: &mut impl Write, tags: TagSet) -> Result<()> {
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

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    dest.write_all(SIGNATURE)?;
    let mut tags = Some(tags);
    let mut byte = read_marker(src)?;
    loop {
        match byte {
            0x00..=0xBF | 0xD8 | 0xF0..=0xFD | 0xFF => return Err(Error::JpegUnknownSegment(byte)),
            // APP0-APP1
            0xE0..=0xE1 => {
                let length_bytes = read_bytes!(src, 2)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2) as u64;
                let content_bytes = read_bytes!(src, length)?;
                dest.write_all(&[0xFF, byte])?;
                dest.write_all(&length_bytes)?;
                dest.write_all(&content_bytes)?;
                if content_bytes.starts_with(match byte {
                    0xE0 => JFIF_ID,
                    0xE1 => EXIF_ID,
                    _ => unreachable!(),
                }) {
                    if let Some(tags) = tags.take() {
                        write_tags_segment(dest, tags)?;
                    }
                }
                byte = read_marker(src)?;
            }
            // APP4
            0xE4 => {
                let length_bytes = read_bytes!(src, 2)?;
                let length = u16::from_be_bytes(length_bytes).saturating_sub(2) as u64;
                let content_bytes = read_bytes!(src, length)?;
                if !content_bytes.starts_with(TAGS_ID) {
                    dest.write_all(&[0xFF, byte])?;
                    dest.write_all(&length_bytes)?;
                    dest.write_all(&content_bytes)?;
                }
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

#[cfg(test)]
make_tests! {"jpeg"}
