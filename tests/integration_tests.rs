use memedb_core::{read_tags, tagset, write_tags};
use std::{fs::File, io::sink, path::Path};

#[test]
fn read() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("images");
    for file in path.read_dir().unwrap() {
        let path = file.unwrap().path();
        let mut file = File::open(path).unwrap();
        read_tags(&mut file).unwrap();
    }
}

#[test]
fn write() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("images");
    for file in path.read_dir().unwrap() {
        let path = file.unwrap().path();
        let mut file = File::open(path).unwrap();
        write_tags(&mut file, &mut sink(), tagset! {}).unwrap();
    }
}
