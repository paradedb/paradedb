#!/bin/bash
#
# Antithesis singleton driver: runs the `qgen` property tests against the
# in-cluster ParadeDB primary in a loop, with concurrency forced on and a
# server-side statement_timeout enforced (see PARADEDB_FORCE_PARALLEL and
# PARADEDB_QGEN_STATEMENT_TIMEOUT_MS in tests/tests/fixtures/querygen/mod.rs).

set -Eeuo pipefail

export DATABASE_URL="postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"
export PARADEDB_FORCE_PARALLEL=1
export PARADEDB_QGEN_STATEMENT_TIMEOUT_MS="${PARADEDB_QGEN_STATEMENT_TIMEOUT_MS:-60000}"
export PROPTEST_CASES="${PROPTEST_CASES:-64}"
# Disable proptest's source-relative regression file persistence: the qgen
# binary runs air-gapped from /home/app and can't resolve lib.rs/main.rs, which
# otherwise spams a warning on every iteration.
export PROPTEST_FAILURE_PERSISTENCE=off
export RUST_BACKTRACE=1

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Starting qgen property test loop..."
iter=0
while true; do
  iter=$((iter + 1))
  echo ""
  echo "=== qgen iteration ${iter} ==="
  /home/app/qgen --nocapture
done
