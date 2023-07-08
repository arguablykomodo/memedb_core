#!/bin/sh
mkdir -p fuzz/corpus/$1
cargo +nightly fuzz run \
  --strip-dead-code --features memedb_core/$1 $2 fuzz/corpus/$1 -- \
  -seed_inputs=tests/media/minimal.${3},tests/media/minimal_empty.${3},tests/media/minimal_tagged.${3} \
  -max_total_time=120
