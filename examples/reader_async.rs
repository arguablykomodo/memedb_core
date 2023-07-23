use futures::StreamExt;
use memedb_core::read_tags_async;
use smol::{fs::File, io::BufReader};

// Reads the file paths provided as args and prints the tags they contain, if any
// `cargo run --example reader_async -- meme.ext`
fn main() {
    smol::block_on(async {
        let ex = smol::LocalExecutor::new();
        let tasks = smol::stream::iter(std::env::args().skip(1).map(|path| {
            ex.spawn(async move {
                let file = File::open(&path).await.unwrap();
                (path, read_tags_async(&mut BufReader::new(file)).await)
            })
        }));
        tasks
            .for_each_concurrent(10, |task| async {
                let (path, result) = ex.run(task).await;
                match result {
                    Ok(tags) => match tags {
                        Some(tags) => println!("{}: {:?}", path, tags),
                        None => println!("{}: unknown format", path),
                    },
                    Err(e) => eprintln!("{}: {}", path, e),
                }
            })
            .await;
    })
}
