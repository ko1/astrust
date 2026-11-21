#!/bin/bash
# reuse_test.sh [SARUBY] : Reusable — a store baked by one program serves another.
# tools/reuse/a.rb bakes a store; b.rb (the same methods at other lines, other code around them) runs with it.
# Passes when b.rb with a.rb's store has program SD hits, prints what it prints with no store, and runs within
# 2% of the instructions it takes with its own store.  Prints the numbers either way.
K=$(realpath "${1:-$(dirname "$0")/../saruby}"); D=$(cd "$(dirname "$0")/reuse" && pwd)
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
cp "$D"/*.rb "$w"/; cd "$w" || exit 1
run() {  # store-dir -> "hits instructions output-md5"
  local o; o=$(perf stat -x, -e instructions:u "$K" --astro-verbose --code-store "$1" b.rb 2>&1 >out.txt)
  local h i; h=$(echo "$o" | sed -n 's/.*sd_hits=\([0-9]*\).*/\1/p'); i=$(echo "$o" | grep instructions | cut -d, -f1)
  case "$h$i" in ''|*[!0-9]*) echo "no count from: $o" >&2; exit 2;; esac
  echo "$h $i $(md5sum < out.txt | cut -c1-32)"
}
"$K" --aot-compile -q --code-store storeA a.rb >/dev/null || { echo "bake of a.rb failed" >&2; exit 2; }
"$K" --aot-compile -q --code-store storeB b.rb >/dev/null || { echo "bake of b.rb failed" >&2; exit 2; }
mkdir empty
read -r h0 i0 m0 <<< "$(run empty)"
read -r ha ia ma <<< "$(run storeA)"
read -r hb ib mb <<< "$(run storeB)"
printf "b.rb  no store: sd_hits %d (prelude), %dM instructions\n" "$h0" $((i0 / 1000000))
printf "b.rb  a.rb's store: sd_hits %d (+%d), %dM\n" "$ha" $((ha - h0)) $((ia / 1000000))
printf "b.rb  its own store: sd_hits %d (+%d), %dM\n" "$hb" $((hb - h0)) $((ib / 1000000))
ok=1
[ "$ha" -gt "$h0" ] || { echo "FAIL: no SD of a.rb's store was used"; ok=0; }
[ "$m0" = "$ma" ] && [ "$m0" = "$mb" ] || { echo "FAIL: output differs between stores"; ok=0; }
[ $((ia * 100)) -le $((ib * 102)) ] || { echo "FAIL: with a.rb's store more than 2% over its own store"; ok=0; }
[ $ok = 1 ] && echo PASS
[ $ok = 1 ]
