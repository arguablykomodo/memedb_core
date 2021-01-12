use crate::{
    error::{Error, Result},
    TagSet,
};
use std::io::{Read, Seek, Write};

pub const SIGNATURE: &[u8] = b"RIFF";

const TAG_CHUNK: &[u8; 4] = b"meme";

// https://en.wikipedia.org/wiki/Interchange_File_Format
//
// RIFF is essentially the same as IFF but the lengths are in little endian. Each chunk has a
// 4-byte ASCII name, a 4-byte little endian length, and then a payload (plus an extra padding byte
// if the length is not even). The file is composed of one big RIFF meta-chunk, that contains a
// 4-byte ASCII name describing the format of the payload (`WEBP`, `AVI `, `WAV `, etc), and then a
// series of subchunks. We store our stuff in the `meme` chunk, and the tags are encoded just like
// we do it in png.
//
// Cool fact: I actually wrote a 400-word rant on how much i hated this format but turns out i
// actually didnt understand how it worked so nevermind all that.

pub fn read_tags(src: &mut (impl Read + Seek)) -> Result<crate::TagSet> {
    let mut file_length = u32::from_le_bytes(read_bytes!(src, 4)) - 4;
    skip_bytes!(src, 4)?;
    while file_length > 0 {
        let name = read_bytes!(src, 4);
        let mut length = u32::from_le_bytes(read_bytes!(src, 4));
        if &name == TAG_CHUNK {
            let mut bytes = read_bytes!(src, length as usize);
            let mut tags = TagSet::new();
            while !bytes.is_empty() {
                let size = bytes.remove(0) as usize;
                let bytes: Vec<u8> = bytes.drain(..size.min(bytes.len())).collect();
                tags.insert(String::from_utf8(bytes)?);
            }
            return Ok(tags);
        }
        // If `length` was 0xFFFFFFFF, adding 1 would cause an overflow and that causes all sorts
        // of issues. Here we do a saturating addition, which you would assume would actually be
        // wrong as we would be off by 1 byte when reading, but turns out that a valid RIFF
        // container will never have a subchunk with 0xFFFFFFFF length, as the maximum length of
        // the container itself is 0xFFFFFFFF, and due to the name and length bytes, a chunk
        // necessarily will have a smaller length than that.
        length = length.saturating_add(length & 1);
        skip_bytes!(src, length as i64)?;
        use std::io::{Error, ErrorKind::UnexpectedEof};
        // Name + length + payload
        match file_length.checked_sub(4 + 4 + length) {
            Some(n) => file_length = n,
            None => return Err(Error::new(UnexpectedEof, "Incorrect chunk length").into()),
        }
    }
    Ok(TagSet::new())
}

pub fn write_tags(src: &mut (impl Read + Seek), dest: &mut impl Write, tags: TagSet) -> Result<()> {
    dest.write_all(SIGNATURE)?;

    let mut file_length = u32::from_le_bytes(read_bytes!(src, 4));

    // Because we need to write the length of the file at the beggining, we need to store
    // everything in a buffer before writing. Those four 0x0 bytes are placeholders for the final
    // length
    let mut buffer = vec![0, 0, 0, 0];

    buffer.extend_from_slice(&read_bytes!(src, 4));
    file_length -= 4;

    while file_length > 0 {
        let name = read_bytes!(src, 4);
        let chunk_length_bytes = read_bytes!(src, 4);
        let mut chunk_length = u32::from_le_bytes(chunk_length_bytes);
        chunk_length += chunk_length & 1;
        if &name == TAG_CHUNK {
            skip_bytes!(src, chunk_length as i64)?;
        } else {
            buffer.extend_from_slice(&name);
            buffer.extend_from_slice(&chunk_length_bytes);
            buffer.extend_from_slice(&read_bytes!(src, chunk_length as usize));
        }
        file_length -= 4 + 4 + chunk_length; // Name + length + payload
    }

    // We have to store the tags at the end because webp wants the chunks to be in a specific order
    // So yeah, thanks webp
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    let tag_bytes = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
        acc.push(tag.len() as u8);
        acc.extend(tag.into_bytes());
        acc
    });

    if tag_bytes.len() as u64 >= std::u32::MAX as u64 {
        return Err(Error::ChunkSizeOverflow);
    }

    let tags_length = tag_bytes.len() as u32;
    buffer.extend_from_slice(TAG_CHUNK);
    buffer.extend(tags_length.to_le_bytes().iter());
    buffer.extend(tag_bytes.into_iter());
    if tags_length & 1 == 1 {
        buffer.push(0);
    }

    // We subtract 4 here because the bytes storing the size are not counted
    let final_length = (buffer.len() as u32 - 4).to_le_bytes();
    buffer[0] = final_length[0]; // THIS
    buffer[1] = final_length[1]; // IS
    buffer[2] = final_length[2]; // VERY
    buffer[3] = final_length[3]; // DUMB

    dest.write_all(&buffer)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tagset;
    use quickcheck_macros::quickcheck;
    use std::io::Cursor;

    #[test]
    fn untagged() {
        assert_read!("minimal.webp", tagset! {});
        assert_write!("minimal.webp", tagset! { "foo", "bar" }, "minimal_tagged.webp");
    }

    #[test]
    fn empty() {
        assert_read!("minimal_empty.webp", tagset! {});
        assert_write!("minimal_empty.webp", tagset! { "foo", "bar" }, "minimal_tagged.webp");
    }

    #[test]
    fn tagged() {
        assert_read!("minimal_tagged.webp", tagset! { "foo", "bar" });
        assert_write!("minimal_tagged.webp", tagset! {}, "minimal_empty.webp");
    }

    #[test]
    fn large() {
        assert_read!("large.webp", tagset! {});
    }

    #[quickcheck]
    #[ignore]
    fn qc_read_never_panics(bytes: Vec<u8>) -> bool {
        let _ = read_tags(&mut Cursor::new(&bytes));
        true
    }

    #[quickcheck]
    #[ignore]
    fn qc_write_never_panics(bytes: Vec<u8>, tags: TagSet) -> bool {
        if crate::are_tags_valid(&tags) {
            let _ = write_tags(&mut Cursor::new(&bytes), &mut std::io::sink(), tags);
        }
        true
    }

    #[quickcheck]
    #[ignore]
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
