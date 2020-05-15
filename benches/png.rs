use criterion::{criterion_group, criterion_main, BatchSize::SmallInput, Criterion};
use memedb_core::{read_tags, tagset, write_tags};
use std::io::{sink, Cursor};

pub fn read(c: &mut Criterion) {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/images/when_you.png"));
    c.bench_function("png read", |b| {
        b.iter_batched(
            || Cursor::new(&bytes[..]),
            |mut src| read_tags(&mut src).unwrap(),
            SmallInput,
        )
    });
}

pub fn write(c: &mut Criterion) {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/images/when_you.png"));
    c.bench_function("png write", |b| {
        b.iter_batched(
            || Cursor::new(&bytes[..]),
            |mut src| write_tags(&mut src, &mut sink(), tagset! {}).unwrap(),
            SmallInput,
        )
    });
}

criterion_group!(png, read, write);
criterion_main!(png);
