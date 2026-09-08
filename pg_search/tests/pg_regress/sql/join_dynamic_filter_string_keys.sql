-- Hash-join InList dynamic filters on string-backed join keys.
--
-- `uuid` and tokenizer-cast text columns both store as Utf8View, the same
-- as a plain `text` field, but only `text` used to convert the join-derived
-- InList into a TermSetQuery. The others fell back to a post-search pre-filter,
-- so the probe side read every document in the index.
--
-- The `dynamic_filter_pushdown_*` token plus a small `rows_scanned` on the probe
-- scan is what proves the term set reached tantivy. A `uuid` foreign key is the
-- common shape here, so it gets the widest coverage.

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan = on;

DROP TABLE IF EXISTS strkey_probe CASCADE;
DROP TABLE IF EXISTS strkey_build CASCADE;

-- The build side keeps 20 of the 5,000 keys, so an index-driven term set reads
-- ~20 documents while the pre-filter fallback reads all 5,000.
CREATE TABLE strkey_build (
    id       uuid PRIMARY KEY,
    id_txt   text NOT NULL,
    keep     boolean NOT NULL,
    ordinal  bigint NOT NULL
);

INSERT INTO strkey_build (id, id_txt, keep, ordinal)
SELECT md5('k' || i)::uuid,
       md5('k' || i),
       i <= 20,
       i
FROM generate_series(1, 5000) i;

CREATE TABLE strkey_probe (
    id        bigserial PRIMARY KEY,
    fk_uuid   uuid NOT NULL,
    fk_txt    text NOT NULL,
    amount    bigint NOT NULL
);

INSERT INTO strkey_probe (fk_uuid, fk_txt, amount)
SELECT md5('k' || i)::uuid,
       md5('k' || i),
       i
FROM generate_series(1, 5000) i;

CREATE INDEX strkey_build_idx ON strkey_build
USING bm25 (id, (id_txt::pdb.literal), keep, ordinal)
WITH (key_field = 'id');

CREATE INDEX strkey_probe_idx ON strkey_probe
USING bm25 (id, fk_uuid, (fk_txt::pdb.literal), amount)
WITH (key_field = 'id');

ANALYZE strkey_build;
ANALYZE strkey_probe;

-- =============================================================================
-- TEST 1: uuid join key
-- =============================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT b.ordinal
FROM strkey_build b
JOIN strkey_probe p ON p.fk_uuid = b.id
WHERE b.id @@@ paradedb.all() AND b.keep = true AND p.amount >= 0
ORDER BY b.ordinal ASC
LIMIT 10;

SELECT b.ordinal
FROM strkey_build b
JOIN strkey_probe p ON p.fk_uuid = b.id
WHERE b.id @@@ paradedb.all() AND b.keep = true AND p.amount >= 0
ORDER BY b.ordinal ASC
LIMIT 10;

-- =============================================================================
-- TEST 2: tokenizer-cast text join key
-- =============================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT b.ordinal
FROM strkey_build b
JOIN strkey_probe p ON p.fk_txt = b.id_txt
WHERE b.id @@@ paradedb.all() AND b.keep = true AND p.amount >= 0
ORDER BY b.ordinal ASC
LIMIT 10;

SELECT b.ordinal
FROM strkey_build b
JOIN strkey_probe p ON p.fk_txt = b.id_txt
WHERE b.id @@@ paradedb.all() AND b.keep = true AND p.amount >= 0
ORDER BY b.ordinal ASC
LIMIT 10;

-- =============================================================================
-- TEST 3: pushdown disabled, so the pre-filter fallback still gives the same rows
-- =============================================================================

SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 0;

SELECT b.ordinal
FROM strkey_build b
JOIN strkey_probe p ON p.fk_uuid = b.id
WHERE b.id @@@ paradedb.all() AND b.keep = true AND p.amount >= 0
ORDER BY b.ordinal ASC
LIMIT 10;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;

DROP TABLE strkey_probe;
DROP TABLE strkey_build;
