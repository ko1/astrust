#!/bin/sh
# Builds the test binary and runs the regex engine test suite against CRuby.
#   ./run.sh            curated cases (gen.rb), leading-.* anchor grid (anystar.rb) + 1M-byte stress tests
#   ./run.sh fuzz N S   additionally N random differential cases with seed S (plain and "rich" grammar)
#   ./run.sh bench      scan throughput on a 1MB text
# RUSTFLAGS overrides the parent crate's prefer-dynamic setting so the binary runs standalone.
set -e
cd "$(dirname "$0")"
export RUSTFLAGS="-C target-cpu=native"
cargo build --release -q
BIN=./target/release/regex_test
if [ "$1" = bench ]; then exec $BIN bench; fi
ruby gen.rb > cases.tsv 2>/dev/null
$BIN < cases.tsv
ruby anystar.rb > anystar.tsv
$BIN < anystar.tsv
$BIN stress
if [ "$1" = fuzz ]; then
  ruby fuzz.rb "${2:-5000}" "${3:-1}" > fuzz.tsv 2>/dev/null
  ruby fuzz.rb "${2:-5000}" "${3:-1}" rich > fuzz_rich.tsv 2>/dev/null
  $BIN < fuzz.tsv | tail -20
  $BIN < fuzz_rich.tsv | tail -20
fi
