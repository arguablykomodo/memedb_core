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
