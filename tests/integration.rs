use memedb_core::{read_tags, write_tags};
use std::{fs::File, io::sink, path::Path};

#[test]
fn read() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("media");
    for file in path.read_dir().unwrap() {
        let path = file.unwrap().path();
        let file = File::open(path).unwrap();
        read_tags(&mut std::io::BufReader::new(file)).unwrap();
    }
}

#[test]
fn write() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("media");
    for file in path.read_dir().unwrap() {
        let path = file.unwrap().path();
        let file = File::open(path).unwrap();
        write_tags(&mut std::io::BufReader::new(file), &mut sink(), &[] as &[&str]).unwrap();
    }
}
