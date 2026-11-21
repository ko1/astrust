#!/bin/bash
# reuse_test.sh [SARUBY] : Reusable — a store baked by one program serves another.
# Each case is a pair in tools/reuse: A bakes a store, B runs with it.
#   a.rb / b.rb          the same methods at other lines, other code around them
#   ivar_a.rb / ivar_b.rb  the same `sum` and `run`, the class's ivars created in the other order (other slots)
# A case passes when B with A's store has program SD hits, prints what it prints with no store, and runs within
# 2% of the instructions it takes with its own store.  Prints the numbers either way.
K=$(realpath "${1:-$(dirname "$0")/../saruby}"); D=$(cd "$(dirname "$0")/reuse" && pwd)
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
cp "$D"/*.rb "$w"/; cd "$w" || exit 1
run() {  # store-dir program -> "hits instructions output-md5"
  local o; o=$(perf stat -x, -e instructions:u "$K" --astro-verbose --code-store "$1" "$2" 2>&1 >out.txt)
  local h i; h=$(echo "$o" | sed -n 's/.*sd_hits=\([0-9]*\).*/\1/p'); i=$(echo "$o" | grep instructions | cut -d, -f1)
  case "$h$i" in ''|*[!0-9]*) echo "no count from: $o" >&2; exit 2;; esac
  echo "$h $i $(md5sum < out.txt | cut -c1-32)"
}
mkdir empty
all=1
pair() {  # A B
  local a=$1 b=$2 ok=1
  rm -rf storeA storeB
  "$K" --aot-compile -q --code-store storeA "$a" >/dev/null || { echo "bake of $a failed" >&2; exit 2; }
  "$K" --aot-compile -q --code-store storeB "$b" >/dev/null || { echo "bake of $b failed" >&2; exit 2; }
  read -r h0 i0 m0 <<< "$(run empty "$b")"
  read -r ha ia ma <<< "$(run storeA "$b")"
  read -r hb ib mb <<< "$(run storeB "$b")"
  printf "%s  no store: sd_hits %d (prelude), %dM instructions\n" "$b" "$h0" $((i0 / 1000000))
  printf "%s  %s's store: sd_hits %d (+%d), %dM\n" "$b" "$a" "$ha" $((ha - h0)) $((ia / 1000000))
  printf "%s  its own store: sd_hits %d (+%d), %dM\n" "$b" "$hb" $((hb - h0)) $((ib / 1000000))
  [ "$ha" -gt "$h0" ] || { echo "FAIL: no SD of $a's store was used"; ok=0; }
  [ "$m0" = "$ma" ] && [ "$m0" = "$mb" ] || { echo "FAIL: output differs between stores"; ok=0; }
  [ $((ia * 100)) -le $((ib * 102)) ] || { echo "FAIL: with $a's store more than 2% over its own store"; ok=0; }
  [ $ok = 1 ] || all=0
}
pair a.rb b.rb
pair ivar_a.rb ivar_b.rb
[ $all = 1 ] && echo PASS
[ $all = 1 ]
