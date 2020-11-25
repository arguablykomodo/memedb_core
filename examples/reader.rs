// Reads the file paths provided as args and prints the tags they contain, if any
// `cargo run --example reader -- meme.ext`
fn main() {
    for path in std::env::args().skip(1) {
        let mut file = std::fs::File::open(&path).unwrap();
        match memedb_core::read_tags(&mut file) {
            Ok(tags) => match tags {
                Some(tags) => println!("{}: {:?}", path, tags),
                None => println!("{}: unknown format", path),
            },
            Err(e) => eprintln!("{}: {}", path, e),
        }
    }
}
