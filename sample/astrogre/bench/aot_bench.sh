#!/bin/sh
# Interpreter vs AOT-specialized search time of `are` (search phase only, from ASTROGRE_VERBOSE).
#   bench/aot_bench.sh FILE [RUNS] [MODE]     MODE: -c (count matching lines, default) or -oc (count matches)
# Prints min and median over RUNS runs for --plain and for the baked code store.
cd "$(dirname "$0")/.." || exit 1
FILE=$1; RUNS=${2:-5}; MODE=${3:--c}
t() { ASTROGRE_VERBOSE=1 ./are $MODE "$@" "$FILE" 2>&1 >/dev/null | sed -n 's/^are: search //p'; }
stat() { ruby -e 'v = $stdin.read.split.map { |x| x.end_with?("ms") ? x.to_f : x.end_with?("µs") ? x.to_f / 1000 : x.to_f * 1000 }.sort; abort "no data" if v.empty?; printf("min %8.2fms  med %8.2fms", v[0], v[v.size / 2])'; }
for p in 'static' 'rb_\w+\(' '\b(if|else|for|while|return)\b' '[0-9]+\.[0-9]+' '^\s*#\s*define\s+[A-Z_]+' '(\w+) = \1' '[a-z]+_t\b' '(?:ab|cd)+e'; do
  rm -rf code_store
  plain=$(for i in $(seq $RUNS); do t --plain "$p"; done | stat)
  ./are $MODE --aot-compile "$p" "$FILE" >/dev/null || exit 1
  aot=$(for i in $(seq $RUNS); do t "$p"; done | stat)
  printf '%-34s plain: %s   aot: %s   (%s)\n' "$p" "$plain" "$aot" "$(./are $MODE "$p" "$FILE")"
done
rm -rf code_store
