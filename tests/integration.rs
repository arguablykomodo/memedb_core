use memedb_core::{read_tags, read_tags_async, write_tags, write_tags_async};
use smol::stream::StreamExt;
use std::{fs::File, io::sink, path::Path};

#[test]
fn read_async() {
    smol::block_on(async {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("media");
        let mut entries = smol::fs::read_dir(&path).await.unwrap();
        while let Some(file) = entries.next().await {
            let path = file.unwrap().path();
            let file = smol::fs::File::open(path).await.unwrap();
            read_tags_async(&mut smol::io::BufReader::new(file)).await.unwrap();
        }
    });
}

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
fn write_async() {
    smol::block_on(async {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("media");
        let mut entries = smol::fs::read_dir(&path).await.unwrap();
        while let Some(file) = entries.next().await {
            let path = file.unwrap().path();
            let file = smol::fs::File::open(path).await.unwrap();
            write_tags_async(
                &mut smol::io::BufReader::new(file),
                &mut smol::io::sink(),
                &[] as &[&str],
            )
            .await
            .unwrap();
        }
    });
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
