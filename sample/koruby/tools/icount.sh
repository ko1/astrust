#!/bin/bash
# icount.sh KORUBY bench... : user-space instruction counts (interp, AOT) per rubyharness bench.
# Use this on a shared/loaded machine where wall time is noisy.
K=$(realpath "$1"); shift
BD=${BENCH_DIR:-$HOME/ruby/astro/sample/rubyharness/bench}
for b in "$@"; do
  d=$(mktemp -d); cp "$BD/$b.rb" "$d/"; cd "$d" || exit 1
  i=$(perf stat -x, -e instructions:u "$K" --plain "$b.rb" 2>&1 >/dev/null | grep instructions | cut -d, -f1)
  "$K" --aot-compile -q "$b.rb" >/dev/null 2>&1 || echo "$b: bake failed" >&2
  a=$(perf stat -x, -e instructions:u "$K" "$b.rb" 2>&1 >/dev/null | grep instructions | cut -d, -f1)
  [ -n "$i" ] && [ -n "$a" ] || { echo "$b: no count" >&2; exit 1; }
  echo "$b interp=$((i/1000000))M aot=$((a/1000000))M"
  cd /; rm -r "$d"
done
