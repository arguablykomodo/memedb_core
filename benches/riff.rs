use criterion::{criterion_group, criterion_main, BatchSize::SmallInput, Criterion};
use memedb_core::{read_tags, tagset, write_tags};
use std::io::{sink, Cursor};

pub fn read(c: &mut Criterion) {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/large.webp"));
    c.bench_function("riff read", |b| {
        b.iter_batched(
            || Cursor::new(&bytes[..]),
            |mut src| read_tags(&mut src).unwrap(),
            SmallInput,
        )
    });
}

pub fn write(c: &mut Criterion) {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/media/large.webp"));
    c.bench_function("riff write", |b| {
        b.iter_batched(
            || Cursor::new(&bytes[..]),
            |mut src| write_tags(&mut src, &mut sink(), tagset! {}).unwrap(),
            SmallInput,
        )
    });
}

criterion_group!(riff, read, write);
criterion_main!(riff);
