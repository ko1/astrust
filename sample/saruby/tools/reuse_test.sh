#!/bin/bash
# reuse_test.sh [SARUBY] : Reusable — a store baked by one program serves another.
# Each case is a pair in tools/reuse, A and B:
#   reuse  (same shape, same profile): B with A's store runs within 2% of B with its own store
#     a.rb / b.rb            the same methods at other lines, other code around them
#     bi_a.rb / bi_b.rb      the same `x.size` site, Array#size in A, Hash#size in B (the bodies follow from the name)
#   shared (same shape, other profile): with one store baked by A and then B, each runs within 2% of its own store
#   (an SD is named by its structure and its profile: B's SD does not replace A's, nor is A's taken for B's)
#   and a copy of B at another path, run with that store, within 2% of B with its own store (hopt_index.txt is keyed
#   by the source text, not the path)
#     ivar_a.rb / ivar_b.rb  the same `sum` and `run`, the class's ivars created in the other order (other slots)
#     poly_a.rb / poly_b.rb  the same `x.m` site, a class whose #m is another method in each
# Every run must print what B (or A) prints with no store.  Prints the numbers either way.
K=$(realpath "${1:-$(dirname "$0")/../saruby}"); D=$(cd "$(dirname "$0")/reuse" && pwd)
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
cp "$D"/*.rb "$w"/; cd "$w" || exit 1
run() {  # store-dir program -> "hits instructions output-md5"
  local o; o=$(perf stat -x, -e instructions:u "$K" --astro-verbose --code-store "$1" "$2" 2>&1 >out.txt)
  local h i; h=$(echo "$o" | sed -n 's/.*sd_hits=\([0-9]*\).*/\1/p'); i=$(echo "$o" | grep instructions | cut -d, -f1)
  case "$h$i" in ''|*[!0-9]*) echo "no count from: $o" >&2; exit 2;; esac
  echo "$h $i $(md5sum < out.txt | cut -c1-32)"
}
bake() { "$K" --aot-compile -q --code-store "$1" "$2" >/dev/null || { echo "bake of $2 failed" >&2; exit 2; }; }
mkdir empty
all=1
pair() {  # A B MODE
  local a=$1 b=$2 ok=1
  rm -rf storeA storeB storeAB
  bake storeA "$a"; bake storeB "$b"
  read -r h0 i0 m0 <<< "$(run empty "$b")"
  read -r ha ia ma <<< "$(run storeA "$b")"
  read -r hb ib mb <<< "$(run storeB "$b")"
  printf "%s  no store: sd_hits %d (prelude), %dM instructions\n" "$b" "$h0" $((i0 / 1000000))
  printf "%s  %s's store: sd_hits %d (+%d), %dM\n" "$b" "$a" "$ha" $((ha - h0)) $((ia / 1000000))
  printf "%s  its own store: sd_hits %d (+%d), %dM\n" "$b" "$hb" $((hb - h0)) $((ib / 1000000))
  [ "$ha" -gt "$h0" ] || { echo "FAIL: no SD of $a's store was used"; ok=0; }
  [ "$m0" = "$ma" ] && [ "$m0" = "$mb" ] || { echo "FAIL: output differs between stores"; ok=0; }
  if [ "$3" = reuse ]; then
    [ $((ia * 100)) -le $((ib * 102)) ] || { echo "FAIL: with $a's store more than 2% over its own store"; ok=0; }
  else
    bake storeAB "$a"; bake storeAB "$b"
    read -r _ jb nb <<< "$(run storeAB "$b")"
    read -r _ ja na <<< "$(run storeA "$a")"
    read -r _ ka pa <<< "$(run storeAB "$a")"
    read -r _ _ qa <<< "$(run empty "$a")"
    printf "%s  store of %s then %s: %dM;  %s own store %dM, that store %dM\n" "$b" "$a" "$b" $((jb / 1000000)) "$a" $((ja / 1000000)) $((ka / 1000000))
    [ "$nb" = "$m0" ] && [ "$pa" = "$qa" ] && [ "$na" = "$qa" ] || { echo "FAIL: output differs between stores"; ok=0; }
    [ $((jb * 100)) -le $((ib * 102)) ] || { echo "FAIL: $b with the shared store more than 2% over its own store"; ok=0; }
    [ $((ka * 100)) -le $((ja * 102)) ] || { echo "FAIL: $a with the shared store more than 2% over its own store"; ok=0; }
    # the program key is the source's content, not its path: a copy of B elsewhere finds B's SD, not A's
    mkdir -p elsewhere; cp "$b" "elsewhere/copy_$b"
    read -r _ jc nc <<< "$(run storeAB "elsewhere/copy_$b")"
    printf "%s  a copy at elsewhere/copy_%s with that store: %dM\n" "$b" "$b" $((jc / 1000000))
    [ "$nc" = "$m0" ] || { echo "FAIL: output differs for the copy"; ok=0; }
    [ $((jc * 100)) -le $((ib * 102)) ] || { echo "FAIL: the copy of $b more than 2% over $b's own store"; ok=0; }
  fi
  [ $ok = 1 ] || all=0
}
pair a.rb b.rb reuse
pair bi_a.rb bi_b.rb reuse
pair ivar_a.rb ivar_b.rb shared
pair poly_a.rb poly_b.rb shared
[ $all = 1 ] && echo PASS
[ $all = 1 ]
