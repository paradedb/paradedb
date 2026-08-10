#!/bin/bash
#
# Antithesis "eventually" command: with the workload interrupted, every ParadeDB index must pass
# pg_search's own integrity checks (pdb.verify_index). The retry loop only absorbs pg_search
# background activity racing the checker (paradedb/paradedb#5913); real corruption fails every
# attempt.

set -Eeuo pipefail

CONN="postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5"

# EXECUTE keeps the pdb reference out of parse time: the vanilla-postgres timeline has no
# pg_search extension, so the DO block must no-op there.
SQL=$(cat <<'EOF'
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
)

for attempt in $(seq 1 30); do
  if output=$(psql "${CONN}" -X -qAt -v ON_ERROR_STOP=1 -c "${SQL}" 2>&1); then
    exit 0
  fi
  echo "verify_index attempt ${attempt}/30: ${output}" >&2
  sleep 2
done

echo "eventually_verify_index_consistency: ParadeDB indexes are still inconsistent" >&2
exit 1
