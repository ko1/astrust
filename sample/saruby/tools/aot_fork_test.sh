#!/bin/bash
# aot_fork_test.sh [SARUBY] [N] : bake each tools/aot/*.rb, then run it N times (patch binding on and
# SARUBY_PATCH=0); every run must exit 0 and print what --plain prints.
# fork_patch_arena_1122.rb: patch binding after fork wrote the parent's code pages (shared memfd).
# ic_reentry_spec_klass_1123.rb: a recursive site's stale spec_klass (`&mut` cache under re-entry).
# spec_klass_refill_1124.rb: a refill kept spec_klass; the refilled entry re-entered the site (baked callee over another entry).
# class_reuse_cache_1124.rb: a dead singleton / module's slot reused by a new class; no cache keyed by its address may hit.
# class_reuse_stress_1125.rb: the same under GC.stress (a class's inspect was Kernel's), and via send / public_send / when.
K=$(realpath "${1:-$(dirname "$0")/../saruby}"); N=${2:-5}; D=$(cd "$(dirname "$0")/aot" && pwd)
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
cp "$D"/*.rb "$w"/; cd "$w" || exit 1
all=1
for f in *.rb; do
  exp=$("$K" --plain "$f" 2>&1) || { echo "FAIL $f: --plain exited $?"; all=0; continue; }
  "$K" --aot-compile -q --code-store "cs_$f" "$f" >/dev/null 2>&1 || { echo "FAIL $f: bake"; all=0; continue; }
  [ -e "cs_$f/all.so" ] || { echo "FAIL $f: no all.so"; all=0; continue; }
  bad=0
  for i in $(seq "$N"); do
    for p in 1 0; do
      out=$(SARUBY_PATCH=$p "$K" --code-store "cs_$f" "$f" 2>&1); rc=$?
      [ $rc = 0 ] && [ "$out" = "$exp" ] || { echo "FAIL $f: run $i SARUBY_PATCH=$p rc=$rc"; bad=1; }
    done
  done
  [ $bad = 0 ] && echo "ok $f ($N runs x 2)" || all=0
done
[ $all = 1 ] && echo PASS
[ $all = 1 ]
