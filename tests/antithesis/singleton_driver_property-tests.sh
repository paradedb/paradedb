#!/bin/bash
#
# Antithesis singleton driver: runs the `qgen` property tests once against the
# in-cluster ParadeDB primary, with concurrency forced on and a server-side
# statement_timeout enforced (see PARADEDB_FORCE_PARALLEL and
# PARADEDB_QGEN_STATEMENT_TIMEOUT_MS in tests/tests/fixtures/querygen/mod.rs).
#
# Antithesis singleton drivers must finish within the test duration; qgen
# already iterates internally (24 tests * PROPTEST_CASES proptest cases each),
# so we run the binary once rather than wrapping it in an outer loop. To extend
# the test surface, raise PROPTEST_CASES rather than re-introducing a loop.

set -Eeuo pipefail

export DATABASE_URL="postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"
export PARADEDB_FORCE_PARALLEL=1
# Hang detector for pathologically slow generated queries (issue #2733), not a fault-tolerance knob.
export PARADEDB_QGEN_STATEMENT_TIMEOUT_MS="${PARADEDB_QGEN_STATEMENT_TIMEOUT_MS:-60000}"
export PROPTEST_CASES="${PROPTEST_CASES:-128}"
# Disable proptest's source-relative regression file persistence: the qgen
# binary runs air-gapped from /home/app and can't resolve lib.rs/main.rs, which
# otherwise spams a warning on every iteration.
export PROPTEST_DISABLE_FAILURE_PERSISTENCE=1
export RUST_BACKTRACE=1
# Poke channel for the recovery-liveness command; see tests/antithesis/anytime_recovery_liveness.sh.
export PARADEDB_QGEN_RECONNECT_GRACE_FILE=/tmp/qgen-reconnect-grace

echo ""
echo "Starting qgen property tests..."
/home/app/qgen --nocapture
