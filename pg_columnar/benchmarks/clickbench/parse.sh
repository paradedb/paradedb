#!/bin/bash

# This script is used to parse the results of the ClickBench benchmark suite into the
# format expected by the ClickBench dashboard. We only use it in the full-suite benchmarking
# run, which is run via Docker. For local benchmarking via pgrx, we simply display the
# results in the terminal instead.

# OS X: you need to use the gnu tools from homebrew
GREP="grep"
if command -v ggrep >/dev/null 2>&1; then
  grep="ggrep"
fi
SED="sed"
if command -v gsed >/dev/null 2>&1; then
  sed="gsed"
fi

$GREP -oP 'Time: \d+\.\d+ ms' | $SED -r -e 's/Time: ([0-9]+\.[0-9]+) ms/\1/' |
awk '{ if (i % 3 == 0) { printf "[" }; printf $1 / 1000; if (i % 3 != 2) { printf "," } else { print "]," }; ++i; }'
