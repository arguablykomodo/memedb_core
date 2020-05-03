macro_rules! read_bytes {
    // Return u8 if reading one byte
    ($src:expr, 1) => {{
        let mut bytes = [0; 1];
        $src.read_exact(&mut bytes)?;
        bytes[0]
    }};
    // Use the stack if the length is known at compile-time
    ($src:expr, $n:literal) => {{
        let mut bytes = [0; $n];
        $src.read_exact(&mut bytes)?;
        bytes
    }};
    // Use the heap otherwise
    ($src:expr, $n:expr) => {{
        let mut bytes = vec![0; $n];
        $src.read_exact(&mut bytes)?;
        bytes
    }};
}

macro_rules! skip_bytes {
    ($src:expr, $n:expr) => {
        $src.seek(std::io::SeekFrom::Current($n))
    };
}

#[cfg(test)]
macro_rules! assert_read {
    ($file:literal, $tags:expr) => {
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $file));
        let mut cursor = std::io::Cursor::new(&bytes[..]);
        cursor.set_position(SIGNATURE.len() as u64);
        assert_eq!(read_tags(&mut cursor).unwrap(), $tags);
    };
}

#[cfg(test)]
macro_rules! assert_write {
    ($file:literal, $tags:expr, $reference:literal) => {
        let src = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $file));
        let mut src = std::io::Cursor::new(&src[..]);
        src.set_position(SIGNATURE.len() as u64);

        let mut dest = Vec::new();
        write_tags(&mut src, &mut dest, $tags).unwrap();

        let reference = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", $reference));

        assert_eq!(&dest[..], &reference[..]);
    };
}
