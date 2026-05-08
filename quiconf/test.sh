#!/usr/bin/env bash
# Compile all test .typ files under tests/ into same-name PDFs.
set -euo pipefail
cd "$(dirname "$0")"

ok=0 fail=0
for src in tests/*.typ; do
  out="tests/$(basename "$src" .typ).pdf"
  if typst compile --root . "$src" "$out"; then
    echo "  OK  $out"
    ok=$((ok + 1))
  else
    echo "FAIL  $src"
    fail=$((fail + 1))
  fi
done

echo "--- $ok ok, $fail failed ---"
[ "$fail" -eq 0 ]
