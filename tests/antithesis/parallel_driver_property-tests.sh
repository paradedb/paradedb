#!/bin/bash
#
# Antithesis test driver: runs the `qgen` property tests against the
# in-cluster ParadeDB primary, with concurrency forced on and a server-side
# statement_timeout enforced (see PARADEDB_FORCE_PARALLEL and
# PARADEDB_QGEN_STATEMENT_TIMEOUT_MS in tests/tests/fixtures/querygen/mod.rs).
#
# The `parallel_driver_` prefix tells the Antithesis composer to re-roll
# this command across the campaign (and to allow multiple concurrent copies
# under fault injection), so each invocation explores a fresh proptest seed
# under a fresh fault-injection trajectory. Each qgen invocation creates
# its own ephemeral test database (see tests/tests/fixtures/db.rs), so
# concurrent copies don't share schema. Keep PROPTEST_CASES low so each
# invocation finishes in ~1-2 minutes — that lets the composer interleave
# plenty of fault injection between runs.

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

# Wait for the ParadeDB primary to accept connections. On the first invocation
# this covers cluster bootstrap (~30-60s); on subsequent invocations it
# returns almost immediately.
echo ""
echo "Waiting for ParadeDB primary to be reachable..."
for i in $(seq 1 120); do
  if psql "$DATABASE_URL" -tAXc 'SELECT 1' >/dev/null 2>&1; then
    echo "ParadeDB primary reachable after ${i}s."
    break
  fi
  if [ "$i" -eq 120 ]; then
    echo "ERROR: ParadeDB primary unreachable after 120s, exiting." >&2
    exit 1
  fi
  sleep 1
done

echo ""
echo "Starting qgen property tests..."
/home/app/qgen --nocapture
