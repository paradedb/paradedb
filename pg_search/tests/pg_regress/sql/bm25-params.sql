-- Tests for per-field BM25 parameter tuning (k1 and b) via typmod syntax

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS bm25_params_test CASCADE;
CREATE TABLE bm25_params_test (
    id INTEGER PRIMARY KEY,
    short_text TEXT,
    long_text TEXT
);

INSERT INTO bm25_params_test (id, short_text, long_text) VALUES
(1, 'search search search', 'search search search extra words here to pad the document length out significantly more'),
(2, 'search engine', 'search engine with many additional filler words to make this document much longer than the others'),
(3, 'database query', 'database query optimization techniques and strategies for improving performance in production'),
(4, 'search', 'search is a common operation in databases and information retrieval systems worldwide today');

-- =============================================================================
-- TEST 1: Default BM25 parameters (k1=1.2, b=0.75) as baseline
-- =============================================================================

CREATE INDEX bm25_default_idx ON bm25_params_test
USING bm25 (id, short_text)
WITH (key_field='id');

SELECT id, short_text, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE short_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_default_idx;

-- =============================================================================
-- TEST 2: k1=0 disables term frequency — all matching docs score the same
-- =============================================================================

CREATE INDEX bm25_low_k1_idx ON bm25_params_test
USING bm25 (id, (short_text::pdb.simple('k1=0.0')))
WITH (key_field='id');

SELECT id, short_text, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE short_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_low_k1_idx;

-- =============================================================================
-- TEST 3: b=0 disables length normalization
-- =============================================================================

CREATE INDEX bm25_no_len_norm_idx ON bm25_params_test
USING bm25 (id, (long_text::pdb.simple('b=0.0')))
WITH (key_field='id');

SELECT id, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE long_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_no_len_norm_idx;

-- =============================================================================
-- TEST 4: b=1.0 maximum length normalization
-- =============================================================================

CREATE INDEX bm25_full_len_norm_idx ON bm25_params_test
USING bm25 (id, (long_text::pdb.simple('b=1.0')))
WITH (key_field='id');

SELECT id, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE long_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_full_len_norm_idx;

-- =============================================================================
-- TEST 5: Per-field parameters — different k1/b on each field
-- =============================================================================

CREATE INDEX bm25_per_field_idx ON bm25_params_test
USING bm25 (
    id,
    (short_text::pdb.simple('k1=0.5', 'b=0.3')),
    (long_text::pdb.simple('k1=1.5', 'b=0.9'))
)
WITH (key_field='id');

SELECT id, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE short_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

SELECT id, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE long_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_per_field_idx;

-- =============================================================================
-- TEST 6: High k1 gives more weight to term frequency
-- =============================================================================

CREATE INDEX bm25_high_k1_idx ON bm25_params_test
USING bm25 (id, (short_text::pdb.simple('k1=5.0')))
WITH (key_field='id');

SELECT id, short_text, round(pdb.score(id)::numeric, 4) AS score
FROM bm25_params_test
WHERE short_text @@@ 'search'
ORDER BY pdb.score(id) DESC, id;

DROP INDEX bm25_high_k1_idx;

-- =============================================================================
-- TEST 7: Validation — b > 1 should error
-- =============================================================================

CREATE INDEX bm25_invalid_idx ON bm25_params_test
USING bm25 (id, (short_text::pdb.simple('b=1.5')))
WITH (key_field='id');

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE bm25_params_test CASCADE;
