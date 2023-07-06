pub const MAGIC: &[u8] = b"ftyp";
pub const OFFSET: usize = 4;

use crate::utils::{read_heap, read_stack, skip};
use crate::TagSet;
use std::io::{Read, Seek, Write};

pub const MEMEDB_UUID: [u8; 16] =
    *b"\x12\xeb\xc6\x4d\xea\x62\x47\xa0\x8e\x92\xb9\xfb\x3b\x51\x8c\x28";

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
        (match self.size {
            Size::Short(s) => s as u64 - 4,
            Size::Long(s) => s - 12,
        }) - (match self.r#type {
            Type::Short(_) => 4,
            Type::Long(_) => 20,
        })
    }
}

pub fn read_tags(src: &mut (impl Read + Seek)) -> crate::Result<crate::TagSet> {
    while let Some(r#box) = Box::read(src).map_or_else(
        |e| if e.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(e) },
        |b| Ok(Some(b)),
    )? {
        if let Size::Short(0) = r#box.size {
            return Ok(TagSet::new());
        }
        match r#box.r#type {
            Type::Long(MEMEDB_UUID) => {
                let mut bytes = read_heap(src, r#box.data_size() as usize)?;
                let mut tags = TagSet::new();
                while !bytes.is_empty() {
                    let size = bytes.remove(0) as usize;
                    let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                    tags.insert(String::from_utf8(bytes)?);
                }
                return Ok(tags);
            }
            _ => skip(src, r#box.data_size() as i64)?,
        };
    }
    Ok(TagSet::new())
}

pub fn write_tags(
    src: &mut (impl Read + Seek),
    dest: &mut impl Write,
    tags: TagSet,
) -> crate::Result<()> {
    while let Some(r#box) = Box::read(src).map_or_else(
        |e| if e.kind() == std::io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(e) },
        |b| Ok(Some(b)),
    )? {
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
        match r#box.r#type {
            Type::Long(MEMEDB_UUID) => {
                skip(src, r#box.data_size() as i64)?;
            }
            _ => {
                r#box.write(dest)?;
                std::io::copy(&mut src.take(r#box.data_size()), dest)?;
            }
        };
    }

    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tags = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.append(&mut tag.into_bytes());
        acc
    });
    let r#box = Box::new(Type::Long(MEMEDB_UUID), tags.len() as u64);
    r#box.write(dest)?;
    dest.write_all(&tags)?;
    Ok(())
}

#[cfg(test)]
make_tests! {"mp4"}
