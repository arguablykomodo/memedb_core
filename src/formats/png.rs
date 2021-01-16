use crate::{
    error::{Error, Result},
    TagSet,
};
use crc::Hasher32;
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = b"\x89PNG\x0D\x0A\x1A\x0A";

const TAG_CHUNK: &[u8; 4] = b"meMe";
const END_CHUNK: &[u8; 4] = b"IEND";

// PNG data is stored in chunks:
// Each chunk starts with a 4 byte big endian number describing the length of the data within.
// After that there's a 4 byte ASCII identifier for the chunk type (meMe in our case).
// Then comes the data, which is as long as the length described.
// We store tags with a size byte, followed by the tag itself.
// And at the end there is a CRC-32 checksum of the chunk type and data.
// An IEND chunk marks the end of the file
// Related links:
// http://www.libpng.org/pub/png/apps/pngcheck.html
// https://en.wikipedia.org/wiki/Portable_Network_Graphics

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            END_CHUNK => return Ok(crate::TagSet::new()),
            TAG_CHUNK => {
                let mut bytes = read_bytes!(src, chunk_length as usize);

                // Verify checksum
                let checksum = u32::from_be_bytes(read_bytes!(src, 4));
                let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
                digest.write(&chunk_type);
                digest.write(&bytes);
                if checksum != digest.sum32() {
                    return Err(Error::PngChecksum);
                }

                // Collect tags
                let mut tags = TagSet::new();
                while !bytes.is_empty() {
                    let size = bytes.remove(0) as usize;
                    let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                    tags.insert(String::from_utf8(bytes)?);
                }
                return Ok(tags);
            }
            // We dont care about these, skip!
            _ => {
                skip_bytes!(src, chunk_length as i64 + 4)?;
            }
        }
    }
}

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    dest.write_all(SIGNATURE)?;

    // The first chunk should always be IHDR, according to the spec, so we are going to read it manually
    let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
    let chunk_type = read_bytes!(src, 4);
    dest.write_all(&chunk_length.to_be_bytes())?;
    dest.write_all(&chunk_type)?;
    dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;

    // Encode tags
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tags = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.append(&mut tag.into_bytes());
        acc
    });

    // If this error is returned, someone has *way* too many tags
    if tags.len() as u64 >= std::u32::MAX as u64 {
        return Err(Error::ChunkSizeOverflow);
    }

    // Compute checksum
    let checksum = {
        let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
        digest.write(TAG_CHUNK);
        digest.write(&tags);
        digest.sum32()
    };

    // Write tag chunk
    let mut buffer = Vec::new();
    buffer.extend(&(tags.len() as u32).to_be_bytes());
    buffer.extend(TAG_CHUNK);
    buffer.extend(tags);
    buffer.extend(&checksum.to_be_bytes());
    dest.write_all(&buffer)?;

    loop {
        let chunk_length = u32::from_be_bytes(read_bytes!(src, 4));
        let chunk_type = read_bytes!(src, 4);
        match &chunk_type {
            // Skip old tags
            TAG_CHUNK => {
                skip_bytes!(src, chunk_length as i64 + 4)?;
            }
            // Write rest of the file
            END_CHUNK => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;
                return Ok(());
            }
            // Leave unrelated chunks unchanged
            _ => {
                dest.write_all(&chunk_length.to_be_bytes())?;
                dest.write_all(&chunk_type)?;
                dest.write_all(&read_bytes!(src, chunk_length as usize + 4))?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tagset;
    use quickcheck_macros::quickcheck;
    use std::io::Cursor;

    #[test]
    fn untagged() {
        assert_read!("minimal.png", tagset! {});
        assert_write!("minimal.png", tagset! { "foo", "bar" }, "minimal_tagged.png");
    }

    #[test]
    fn empty() {
        assert_read!("minimal_empty.png", tagset! {});
        assert_write!("minimal_empty.png", tagset! { "foo", "bar" }, "minimal_tagged.png");
    }

    #[test]
    fn tagged() {
        assert_read!("minimal_tagged.png", tagset! { "foo", "bar" });
        assert_write!("minimal_tagged.png", tagset! {}, "minimal_empty.png");
    }

    #[test]
    fn multiple_chunks() {
        assert_read!("minimal_multiple.png", tagset! { "baz" });
        assert_write!("minimal_multiple.png", tagset! { "foo", "bar" }, "minimal_tagged.png");
    }

    #[test]
    fn large() {
        assert_read!("large.png", tagset! {});
    }

    #[quickcheck]
    fn qc_read_never_panics(bytes: Vec<u8>) -> bool {
        let _ = read_tags(&mut Cursor::new(&bytes));
        true
    }

    #[quickcheck]
    fn qc_write_never_panics(bytes: Vec<u8>, tags: TagSet) -> bool {
        if crate::are_tags_valid(&tags) {
            let _ = write_tags(&mut Cursor::new(&bytes), &mut std::io::sink(), tags);
        }
        true
    }

    #[quickcheck]
    fn qc_identity(bytes: Vec<u8>, tags: TagSet) -> bool {
        if crate::are_tags_valid(&tags) && read_tags(&mut Cursor::new(&bytes)).is_ok() {
            let mut dest = Vec::new();
            write_tags(&mut Cursor::new(bytes), &mut dest, tags.clone()).unwrap();
            let mut cursor = Cursor::new(dest);
            cursor.set_position(SIGNATURE.len() as u64);
            read_tags(&mut cursor).unwrap() == tags
        } else {
            true
        }
    }
}
