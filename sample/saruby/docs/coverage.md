# Source coverage of saruby's Rust code

`make COVERAGE=1` builds with `-C instrument-coverage` into `target-cov/` and copies `cov/saruby` +
`cov/libsaruby.so` (the binary and the dylib; both carry counters).  `./saruby`, `target/` and the default
build are untouched.  The Makefile rule only exists next to the normal one; with `COVERAGE` unset nothing changes.

`tools/coverage.sh` drives it (see its header):

    tools/coverage.sh build
    COV_WORK=/scratch tools/coverage.sh hand                       # rubyharness t/hand -> hand.profdata
    COV_WORK=/scratch tools/coverage.sh hand ~/ruby/rubyharness/t/cov cov
    COV_WORK=/scratch COV_NAME=lang tools/coverage.sh spec language core/array
    COV_WORK=/scratch tools/coverage.sh merge /scratch/all.profdata /scratch/hand.profdata /scratch/cov.profdata
    COV_WORK=/scratch COV_OUT=report tools/coverage.sh report all  # summary.txt / summary.json / functions.txt / full.json

Each process writes `%p-%m.profraw` (one for the binary, one for the dylib, about 1.5 MB each); the script merges and
deletes them every `COV_CHUNK` files, so the scratch dir stays small.  AOT-baked code and prelude Ruby are not covered.

The trial `~/ruby/src/trials/2026-11-25-saruby-coverage/` has `bin/analyze.rb` (per-file table) and `bin/uncovered.rb`
(largest uncovered line runs) over the llvm-cov export, and the baseline numbers.

Workflow for filling gaps: take the largest uncovered runs in a builtin, write a plain Ruby test in
`~/ruby/rubyharness/t/cov/*_1125c.rb` (prints each case's value or exception, no saruby-specific API), generate the
`.out` with CRuby, run `t/cov/run_suite.sh <interp>`; a difference is a bug (or a CRuby change).
