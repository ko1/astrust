#!/bin/bash
# Source coverage of saruby's Rust code (docs/coverage.md).  Stages (each independent, resumable):
#   tools/coverage.sh build                  make COVERAGE=1 -> cov/saruby (never touches ./saruby)
#   tools/coverage.sh hand [DIR [NAME]]      run DIR/*.rb (default: rubyharness t/hand) with cov/saruby -> NAME.profdata
#   tools/coverage.sh spec REL...            run rubyspec dirs (e.g. core/array language) via sweep.rb
#   tools/coverage.sh report NAME            report from $COV_WORK/NAME.profdata (summary + per-function)
#   tools/coverage.sh merge OUT IN...        merge named profdata into OUT.profdata
# env: COV_WORK (scratch, default $TMPDIR/saruby-cov), COV_OUT (report dir, default ./coverage-report),
#      COV_DIR (dir of the cov build, default ./cov), COV_JOBS (4), COV_TIMEOUT (per file, s; 120), COV_CHUNK (files per merge, 100), SWEEP (sweep.rb path)
# Profiles are written one per process (%p) and merged + deleted per chunk, so disk stays small.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
LLVM=${LLVM_BIN:-$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin}
WORK=${COV_WORK:-${TMPDIR:-/tmp}/saruby-cov}
OUT=${COV_OUT:-$here/coverage-report}
JOBS=${COV_JOBS:-4}
TMO=${COV_TIMEOUT:-120}
CHUNK=${COV_CHUNK:-100}
SWEEP=${SWEEP:-$HOME/ruby/src/trials/2026-11-22-saruby-rubyspec/bin/sweep.rb}
HAND=${HAND_DIR:-$HOME/ruby/rubyharness/t/hand}
COVDIR=${COV_DIR:-$here/cov}   # where saruby + libsaruby.so of the profiled build live (keep a copy to report an older build)
BIN=$COVDIR/saruby
SO=$COVDIR/libsaruby.so
mkdir -p "$WORK" "$OUT"

merge_raw() { # merge_raw DIR OUT.profdata : fold DIR/*.profraw (and an existing OUT) into OUT, delete the raws
  local d=$1 o=$2 in=()
  [ -f "$o" ] && in+=("$o")
  compgen -G "$d/*.profraw" >/dev/null || return 0
  "$LLVM/llvm-profdata" merge -sparse "${in[@]}" "$d"/*.profraw -o "$o.new"
  mv "$o.new" "$o"
  rm -f "$d"/*.profraw
}

case ${1:-} in
build)
  (cd "$here" && make COVERAGE=1) ;;
hand)
  dir=${2:-$HAND}; name=$(basename "${3:-hand}")
  raw=$WORK/raw-$name; mkdir -p "$raw"; rm -f "$raw"/*.profraw
  ls "$dir"/*.rb > "$WORK/$name.list"
  split -l "$CHUNK" "$WORK/$name.list" "$WORK/$name.chunk."
  for c in "$WORK/$name.chunk."*; do
    xargs -a "$c" -P "$JOBS" -I{} bash -c 'cd "$(dirname {})" && LLVM_PROFILE_FILE="$0/%p-%m.profraw" timeout -s KILL "$1" "$2" "$(basename {})" </dev/null >/dev/null 2>&1 || true' "$raw" "$TMO" "$BIN"
    merge_raw "$raw" "$WORK/$name.profdata"
    echo "hand chunk $(basename "$c") done"
    rm -f "$c"
  done ;;
spec)
  shift; name=${COV_NAME:-spec}
  raw=$WORK/raw-$name; mkdir -p "$raw"; rm -f "$raw"/*.profraw
  for rel in "$@"; do
    LLVM_PROFILE_FILE="$raw/%p-%m.profraw" SPEC_TIMEOUT=$TMO ruby "$SWEEP" "$BIN" "$JOBS" "$WORK/$name-$(echo "$rel" | tr / _).tsv" "$rel" || true
    merge_raw "$raw" "$WORK/$name.profdata"
    echo "spec $rel done"
  done ;;
merge)
  out=$2; shift 2
  "$LLVM/llvm-profdata" merge -sparse "$@" -o "$out" ;;
report)
  name=$2; pd=$WORK/$name.profdata
  [ -f "$pd" ] || { echo "no $pd" >&2; exit 1; }
  common=(-instr-profile="$pd" "$BIN" -object "$SO" -ignore-filename-regex='(/\.cargo/|/rustc/|/vendor/|/src/node_|prism_ast|enc_tables|case_tables)' -sources "$here/src")
  "$LLVM/llvm-cov" report "${common[@]}" > "$OUT/$name.summary.txt"
  "$LLVM/llvm-cov" report "${common[@]}" -show-functions >  "$OUT/$name.functions.txt" 2>/dev/null || true
  "$LLVM/llvm-cov" export "${common[@]}" -summary-only > "$OUT/$name.summary.json"
  "$LLVM/llvm-cov" export "${common[@]}" > "$OUT/$name.full.json"
  echo "wrote $OUT/$name.*" ;;
*)
  sed -n '2,11p' "$0"; exit 2 ;;
esac
