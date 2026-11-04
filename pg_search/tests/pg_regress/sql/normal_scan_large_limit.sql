-- Tests that NormalScanExecState supplies every match to a Sort even with a
-- LIMIT larger than its internal batch size (8192). The unsupported text tiebreak
-- prevents TopK, so the outer Limit applies after the full sort.
\i common/common_setup.sql
SET paradedb.planner_warnings = 'off';

DROP TABLE IF EXISTS normal_large_limit CASCADE;
CREATE TABLE normal_large_limit (id TEXT PRIMARY KEY, body TEXT);
INSERT INTO normal_large_limit
SELECT 'k' || g, 'hello world ' || (g % 7) FROM generate_series(1, 20000) g;
CREATE INDEX normal_large_limit_idx ON normal_large_limit USING bm25 (id, body);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) FROM normal_large_limit
WHERE body @@@ 'hello'
ORDER BY pdb.score(id) DESC, id DESC
LIMIT 100000;

-- LIMIT just above the batch size
SELECT count(*) FROM (
    SELECT id, pdb.score(id) FROM normal_large_limit
    WHERE body @@@ 'hello'
    ORDER BY pdb.score(id) DESC, id DESC
    LIMIT 12000
) s;

-- LIMIT above the number of matching rows
SELECT count(*) FROM (
    SELECT id, pdb.score(id) FROM normal_large_limit
    WHERE body @@@ 'hello'
    ORDER BY pdb.score(id) DESC, id DESC
    LIMIT 100000
) s;

-- LIMIT near usize::MAX must not overflow the batch size computation
SELECT count(*) FROM (
    SELECT id, pdb.score(id) FROM normal_large_limit
    WHERE body @@@ 'hello'
    ORDER BY pdb.score(id) DESC, id DESC
    LIMIT 9223372036854775807
) s;

DROP TABLE normal_large_limit CASCADE;
RESET paradedb.planner_warnings;
\i common/common_cleanup.sql
