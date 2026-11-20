#!/bin/sh
# Run every saved fuzz TSV in $1 (default $TMPDIR/rx) and report failing files.
d=${1:-$TMPDIR/rx}
for f in "$d"/*.tsv; do
  timeout 20 "$(dirname "$0")/target/release/regex_test" < "$f" > "$d/out.txt"; r=$?
  if [ $r != 0 ]; then echo "$f $r"; grep -A2 FAIL "$d/out.txt" | head -6; fi
done
