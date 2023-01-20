macro_rules! read_bytes {
    // Return u8 if reading one byte
    ($src:expr, 1) => {{
        let mut byte = 0;
        $src.read_exact(std::slice::from_mut(&mut byte)).map(|_| byte)
    }};
    // Use the stack if the length is known at compile-time
    ($src:expr, $n:literal) => {{
        let mut bytes = [0; $n];
        $src.read_exact(&mut bytes).map(|_| bytes)
    }};
    // Use the heap otherwise
    ($src:expr, $n:expr) => {{
        let mut bytes = Vec::new();
        $src.take($n).read_to_end(&mut bytes).and_then(|n| {
            if (n != $n as usize) {
                Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
            } else {
                Ok(bytes)
            }
        })
    }};
}

macro_rules! skip_bytes {
    ($src:expr, $n:expr) => {
        $src.seek(std::io::SeekFrom::Current($n))
    };
}

#[cfg(test)]
macro_rules! test_file {
    ($name:literal, $ext:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/", $name, ".", $ext))
    };
}

#[cfg(test)]
macro_rules! make_tests {
    ($ext:literal) => {
        mod tests {
            use super::*;
            use crate::tagset;
            use quickcheck_macros::quickcheck;
            use std::io::Cursor;

            const UNTAGGED: &[u8] = test_file!("minimal", $ext);
            const EMPTY: &[u8] = test_file!("minimal_empty", $ext);
            const TAGGED: &[u8] = test_file!("minimal_tagged", $ext);
            const LARGE: &[u8] = test_file!("large", $ext);

            #[test]
            fn untagged() {
                assert_read!(UNTAGGED, tagset! {});
                assert_write!(UNTAGGED, tagset! { "foo", "bar" }, TAGGED);
            }

            #[test]
            fn empty() {
                assert_read!(EMPTY, tagset! {});
                assert_write!(EMPTY, tagset! { "foo", "bar" }, TAGGED);
            }

            #[test]
            fn tagged() {
                assert_read!(TAGGED, tagset! { "foo", "bar" });
                assert_write!(TAGGED, tagset! {}, EMPTY);
            }

            #[test]
            fn large() {
                assert_read!(LARGE, tagset! {});
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
    };
}

#[cfg(test)]
macro_rules! assert_read {
    ($bytes:expr, $tags:expr) => {
        let mut cursor = std::io::Cursor::new($bytes);
        cursor.set_position(SIGNATURE.len() as u64);
        assert_eq!(read_tags(&mut cursor).unwrap(), $tags);
    };
}

// Mix of ascii and unicode control pictures
#[cfg(test)]
#[rustfmt::skip]
const MAPPINGS: [&str; 256] = [
    "␀","␁","␂","␃","␄","␅","␆","␇","␈","␉","␊","␋","␌","␍","␎","␏",
    "␐","␑","␒","␓","␔","␕","␖","␗","␘","␙","␚","␛","␜","␝","␞","␟",
    " ","!","\"","#","$","%","&","'","(",")","*","+",",","-",".","/",
    "0","1","2","3","4","5","6","7","8","9",":",";","<","=",">","?",
    "@","A","B","C","D","E","F","G","H","I","J","K","L","M","N","O",
    "P","Q","R","S","T","U","V","W","X","Y","Z","[","\\","]","^","_",
    "`","a","b","c","d","e","f","g","h","i","j","k","l","m","n","o",
    "p","q","r","s","t","u","v","w","x","y","z","{","|","}","~","␡",
    "Ç","ü","é","â","ä","à","å","ç","ê","ë","è","ï","î","ì","Ä","Å",
    "É","æ","Æ","ô","ö","ò","û","ù","ÿ","Ö","Ü","ø","£","Ø","×","ƒ",
    "á","í","ó","ú","ñ","Ñ","ª","º","¿","®","¬","½","¼","¡","«","»",
    "░","▒","▓","│","┤","Á","Â","À","©","╣","║","╗","╝","¢","¥","┐",
    "└","┴","┬","├","─","┼","ã","Ã","╚","╔","╩","╦","╠","═","╬","¤",
    "ð","Ð","Ê","Ë","È","ı","Í","Î","Ï","┘","┌","█","▄","¦","Ì","▀",
    "Ó","ß","Ô","Ò","õ","Õ","µ","þ","Þ","Ú","Û","Ù","ý","Ý","¯","´",
    "¬","±","‗","¾","¶","§","÷","¸","°","¨","•","¹","³","²","■","␣",
];

#[cfg(test)]
pub(crate) fn hexdump(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{:07X}: {:48}  {}",
                i * 16,
                c.chunks(8)
                    .map(|c| c.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "))
                    .collect::<Vec<_>>()
                    .join("  "),
                c.iter().map(|&c| MAPPINGS[c as usize]).collect::<Vec<_>>().join("")
            )
        })
        .fold(String::new(), |s, l| s + &l + "\n")
}

#[cfg(test)]
macro_rules! assert_write {
    ($bytes:expr, $tags:expr, $reference:expr) => {
        let mut src = std::io::Cursor::new($bytes);
        src.set_position(SIGNATURE.len() as u64);

        let mut dest = Vec::new();
        write_tags(&mut src, &mut dest, $tags).unwrap();

        if (dest != $reference) {
            use crate::utils::hexdump;
            panic!("\nExpected:\n{}Got:\n{}", hexdump($reference), hexdump(&dest));
        }
    };
}
