#!/bin/bash
# sdsize.sh [SARUBY] : bytes of baked code per occurrence of a construct (a method with 22 copies minus one with 2, / 20).
S=$(realpath ${1:-$(dirname $0)/../saruby}); W=$(mktemp -d); cd $W
gen() { ruby -e 'name, line = ARGV; puts "class C; attr_accessor :p; def initialize; @a = 1; @b = [1,2,3]; @p = 2; @o = self; end; def h = 1; def h1(x) = x"; [2, 22].each { |n| puts "def m#{n}(x); s = 0; #{([line] * n).join("; ")}; s; end" }; puts "end; c = C.new; 3.times { c.m2(1); c.m22(1) }"' "$1" "$2" > $1.rb; }
gen lvar 's = x'; gen ivar_get 's = @a'; gen ivar_set '@a = x'; gen opassign_ivar '@a += 1'; gen add 's = s + x'; gen cmp_if 's = 1 if x < s'
gen aref 's = @b[x]'; gen aset '@b[x] = s'; gen fcall0 's = h'; gen fcall1 's = h1(x)'; gen attr_r 's = @o.p'; gen attr_w '@o.p = x'; gen cfunc 's = x.abs'; gen yieldb 's = [x].each { |v| v }'
for f in *.rb; do b=${f%.rb}; mkdir $b.d; cp $f $b.d/; (cd $b.d && $S --aot-compile -q $f >/dev/null 2>&1
  SARUBY_SD_NAMES=1 $S --plain $f 2>&1 | grep -E " m(2|22) " | awk '{print $1, $2}' > names; nm -S --size-sort code_store/all.so | awk '{print $2, $4}' > sizes
  a=$(grep "$(grep ' m2$' names | cut -d' ' -f1)$" sizes | cut -d' ' -f1); c=$(grep "$(grep ' m22$' names | cut -d' ' -f1)$" sizes | cut -d' ' -f1)
  [ -n "$a" ] && [ -n "$c" ] || { echo "$b: no size" >&2; exit 1; }; printf '%-14s %5d bytes each\n' $b $(( (0x$c - 0x$a) / 20 ))); done
cd /; rm -r $W
