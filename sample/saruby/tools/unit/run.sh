#!/bin/bash
# run.sh [SARUBY] : AOT instructions per loop iteration of each unit bench (u_*.rb), minus the empty loop.
# Each bench runs its operation `ops` times per iteration (see the table below); instructions are user-space.
K=$(realpath ${1:-$(dirname $0)/../../saruby}); HERE=$(cd $(dirname $0) && pwd)
declare -A OPS=([u_ivar_get]=5 [u_ivar_set]=5 [u_ivar_arith]=5 [u_cond_ivar]=1 [u_fcall0]=5 [u_attr]=3 [u_step_block]=1 [u_ivar_in_block]=5 [u_fcall0_in_block]=5 [u_fiber]=1 [u_ary_push]=1 [u_ary_aref]=3 [u_ary_aset]=2 [u_send_splat]=1)
ins() { d=$(mktemp -d); cp $HERE/$1.rb $d/; cd $d; $K --aot-compile -q $1.rb >/dev/null 2>&1; perf stat -x, -e instructions:u $K $1.rb 2>&1 >/dev/null | grep instructions | cut -d, -f1; cd /; rm -r $d; }
e=$(ins u_empty); [ -n "$e" ] || { echo "no count for u_empty" >&2; exit 1; }
for b in u_ivar_get u_ivar_set u_ivar_arith u_cond_ivar u_fcall0 u_attr u_step_block u_ivar_in_block u_fcall0_in_block u_fiber u_ary_push u_ary_aref u_ary_aset u_send_splat; do
  n=3000000; [ $b = u_fiber ] && n=600000; i=$(ins $b); [ -n "$i" ] || { echo "no count for $b" >&2; exit 1; }
  ruby -e 'b, i, e, n, k = ARGV; per = (i.to_f - e.to_f * (b == "u_fiber" ? 0 : 1)) / n.to_i; printf("%-20s %7.1f instr/iter  (%d ops: %.1f each)\n", b, per, k.to_i, per / k.to_i)' $b $i $e $n ${OPS[$b]}
done
