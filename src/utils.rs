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
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/", $file));
        let mut cursor = std::io::Cursor::new(bytes);
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
    ($file:literal, $tags:expr, $reference:literal) => {
        let src = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/", $file));
        let mut src = std::io::Cursor::new(src);
        src.set_position(SIGNATURE.len() as u64);

        let mut dest = Vec::new();
        write_tags(&mut src, &mut dest, $tags).unwrap();

        let reference =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/", $reference));

        if (dest != reference) {
            use crate::utils::hexdump;
            panic!("\n{}:\n{}Got:\n{}", $reference, hexdump(reference), hexdump(&dest));
        }
    };
}
