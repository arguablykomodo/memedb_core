// Writes the tags provided as args to the given path
// `cargo run --example writer -- meme.ext foo bar`
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap();
    let tags: Vec<String> = args.collect();

    use std::io::Read;
    let mut file = std::fs::File::open(&path).unwrap();
    let mut buffer = Vec::with_capacity(file.metadata().unwrap().len() as usize);
    file.read_to_end(&mut buffer).unwrap();

    memedb_core::write_tags(
        &mut std::io::Cursor::new(buffer),
        &mut std::fs::File::create(path).unwrap(),
        tags,
    )
    .unwrap();
}
