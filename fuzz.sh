#!/bin/sh
mkdir -p fuzz/corpus/$1
cargo +nightly fuzz run \
  --strip-dead-code --features memedb_core/$1 $2 fuzz/corpus/$1 -- \
  -seed_inputs=tests/media/minimal.${1},tests/media/minimal_empty.${1},tests/media/minimal_tagged.${1} \
  -max_total_time=150
