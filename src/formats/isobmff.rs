//! # ISO Base Media File Format
//!
//! ISOBMFF data is organized in boxes. Each box is structured as follows:
//!
//! - 4 byte big-endian number describing the length of the data within.
//! - 4 byte identifier for the box type.
//! - if the size is 1, then the size is actually stored in the next 8 bytes.
//! - if the size is 0, then the box lasts until the end of the file.
//! - if the type is `uuid`, then the box type is actually stored in the next 12 bytes.
//! - The box data itself, which may consist of other boxes.
//!
//! An ISOBMFF file consists of a series of boxes, the first of which must be of the type `ftyp`.
//!
//! MemeDB stores its tags in a `uuid` box with the UUID `12EBC64DEA6247A08E92B9FB3B518C28`. The
//! box is placed at the end of the file since boxes can reference data via byte offset.
//!
//! ## Relevant Links
//!
//! - [Wikipedia article for ISOBMFF](https://en.wikipedia.org/wiki/ISO_base_media_file_format)
//! - [ISO/IEC 14496-12 standard](https://www.iso.org/standard/83102.html)

pub(crate) const MAGIC: &[u8] = b"ftyp";
pub(crate) const OFFSET: usize = 4;

use crate::{
    utils::{decode_tags, encode_tags, or_eof, passthrough, read_stack, skip},
    Error,
};
use std::io::{Read, Seek, Write};

const MEMEDB_UUID: [u8; 16] = *b"\x12\xeb\xc6\x4d\xea\x62\x47\xa0\x8e\x92\xb9\xfb\x3b\x51\x8c\x28";

#[derive(Debug)]
enum Size {
    Short(u32),
    Long(u64),
}

#[derive(Debug)]
enum Type {
    Short([u8; 4]),
    Long([u8; 16]),
}

#[derive(Debug)]
struct Box {
    size: Size,
    r#type: Type,
}

impl Box {
    fn new(r#type: Type, data_size: u64) -> Self {
        let type_size = match r#type {
            Type::Short(_) => 4,
            Type::Long(_) => 4 + 16,
        };
        let total_size = 4 + type_size + data_size;
        let size = if total_size > u32::MAX.into() {
            Size::Long(total_size + 8)
        } else {
            Size::Short(total_size as u32)
        };
        Self { size, r#type }
    }

    fn read(src: &mut impl Read) -> Result<Box, std::io::Error> {
        let short_size = u32::from_be_bytes(read_stack::<4>(src)?);
        let short_type = read_stack::<4>(src)?;
        let r#box = Box {
            size: match short_size {
                1 => Size::Long(u64::from_be_bytes(read_stack::<8>(src)?)),
                _ => Size::Short(short_size),
            },
            r#type: match &short_type {
                b"uuid" => Type::Long(read_stack::<16>(src)?),
                _ => Type::Short(short_type),
            },
        };
        Ok(r#box)
    }

    fn write(&self, dest: &mut impl Write) -> Result<(), std::io::Error> {
        match self.size {
            Size::Short(s) => dest.write_all(&s.to_be_bytes())?,
            Size::Long(_) => dest.write_all(&[0, 0, 0, 1])?,
        }
        match self.r#type {
            Type::Short(t) => dest.write_all(&t)?,
            Type::Long(_) => dest.write_all(b"uuid")?,
        };
        if let Size::Long(s) = self.size {
            dest.write_all(&s.to_be_bytes())?;
        }
        if let Type::Long(t) = self.r#type {
            dest.write_all(&t)?;
        }
        Ok(())
    }

    fn data_size(&self) -> u64 {
        let type_size = match self.r#type {
            Type::Short(_) => 4,
            Type::Long(_) => 20,
        };
        // Prevents panic when box size is impossibly small, will instead silently pass through.
        match self.size {
            Size::Short(s) => (s as u64).saturating_sub(4 + type_size),
            Size::Long(s) => s.saturating_sub(12 + type_size),
        }
    }
}

/// Given a `src`, return the tags contained inside.
pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<Vec<String>, Error> {
    while let Some(r#box) = or_eof(Box::read(src))? {
        if let Size::Short(0) = r#box.size {
            return Ok(Vec::new());
        }
        if let Type::Long(MEMEDB_UUID) = r#box.r#type {
            return decode_tags(src);
        }
        let size = r#box.data_size();
        // We passthrough instead of skip to get number of bytes read
        if passthrough(src, &mut std::io::sink(), size)? != size {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))?;
        };
    }
    Ok(Vec::new())
}

/// Read data from `src`, set the provided `tags`, and write to `dest`.
///
/// This function will remove any tags that previously existed in `src`.
pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    while let Some(r#box) = or_eof(Box::read(src))? {
        if let Size::Short(0) = r#box.size {
            let pos = src.stream_position()?;
            let len = src.seek(std::io::SeekFrom::End(0))?;
            if pos != len {
                src.seek(std::io::SeekFrom::Start(pos))?;
            }
            Box::new(r#box.r#type, len - pos).write(dest)?;
            std::io::copy(src, dest)?;
            break;
        }
        if let Type::Long(MEMEDB_UUID) = r#box.r#type {
            skip(src, r#box.data_size() as i64)?;
        } else {
            r#box.write(dest)?;
            passthrough(src, dest, r#box.data_size())?;
        };
    }

    let mut tag_bytes = Vec::new();
    encode_tags(tags, &mut tag_bytes)?;
    let r#box = Box::new(Type::Long(MEMEDB_UUID), tag_bytes.len() as u64);
    r#box.write(dest)?;
    dest.write_all(&tag_bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const ZERO_BOX: &[&[u8]] = &[&0u32.to_be_bytes(), &[0; 8]];
    const SIZED_BOX: &[&[u8]] = &[&12u32.to_be_bytes(), &[0; 8]];
    const TAGS: &[&[u8]] = &[&26u32.to_be_bytes(), b"uuid", &MEMEDB_UUID, &[0x80, 0x00]];

    #[test]
    fn size_zero_box() {
        let src = &ZERO_BOX.concat();
        assert_eq!(read_tags(&mut Cursor::new(src)).unwrap(), Vec::<String>::new());
        let mut dest = Vec::new();
        write_tags(&mut Cursor::new(src), &mut dest, vec![""]).unwrap();
        let expected = &[SIZED_BOX.concat(), TAGS.concat()].concat();
        assert_eq!(&dest, expected);
    }
}

crate::utils::standard_tests!("mp4");
