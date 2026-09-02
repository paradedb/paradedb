#!/bin/bash
#
# Antithesis "eventually" command: with the workload killed and faults stopped, every ParadeDB
# index must pass pg_search's own integrity checks (pdb.verify_index). Faults stop when an
# eventually command starts, but recovery takes time (a killed postgres pod needs a restart and
# WAL replay), so the deadline loop rides out connection errors as well as pg_search background
# activity racing the checker (paradedb/paradedb#5913); real corruption fails every attempt
# until the deadline.

set -Eeuo pipefail

CONN="postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5"

# EXECUTE keeps the pdb reference out of parse time when a topology has not installed pg_search
# in the database being inspected, so the DO block can safely no-op there.
read -r -d '' SQL <<'EOF' || true
DO $$
DECLARE
    bad text;
BEGIN
    IF NOT EXISTS (SELECT FROM pg_extension WHERE extname = 'pg_search') THEN
        RETURN;
    END IF;
    EXECUTE $q$
        SELECT string_agg(v.check_name || COALESCE(': ' || v.details, ''), E'\n')
          FROM (SELECT c.oid::regclass AS idx
                  FROM pg_class c JOIN pg_am am ON am.oid = c.relam
                 -- The AM is named paradedb; bm25 remains as a legacy alias, and indexes
                 -- created USING bm25 carry that AM's oid, so match both.
                 WHERE am.amname IN ('paradedb', 'bm25')) i,
               LATERAL pdb.verify_index(i.idx) v
         WHERE NOT v.passed
    $q$ INTO bad;
    IF bad IS NOT NULL THEN
        RAISE EXCEPTION 'ParadeDB index consistency check failed: %', bad;
    END IF;
END
$$;
EOF

# The deadline is 1.5x the workload's own 200s reconnect-grace: that grace holds under active
# fault injection, while this command runs with all faults stopped, so a database that is still
# unreachable at the deadline is itself a recovery-liveness failure worth reporting.
DEADLINE=$((SECONDS + 300))
output=""
while (( SECONDS < DEADLINE )); do
  if output=$(psql "${CONN}" -X -qAt -v ON_ERROR_STOP=1 -c "${SQL}" 2>&1); then
    exit 0
  fi
  echo "verify_index retry (t=${SECONDS}s): ${output}" >&2
  sleep 5
done

echo "eventually_verify_index_consistency failed: ${output}" >&2
exit 1
