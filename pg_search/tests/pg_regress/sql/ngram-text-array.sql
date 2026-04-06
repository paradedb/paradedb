\i common/common_setup.sql

-- Test: ngram match + conjunction_mode on TEXT[] columns

CREATE TABLE books (
    id SERIAL PRIMARY KEY,
    all_titles TEXT[]
);

INSERT INTO books (all_titles) VALUES
    (ARRAY['The Dragon Hatchling', 'A Tale of Fire', 'Wings of Gold']),
    (ARRAY['PostgreSQL Database Guide', 'SQL for Beginners', 'Advanced Queries']),
    (ARRAY['The Hatchling Returns', 'Dragon Slayer', 'Fire and Ice']),
    (ARRAY['Cooking with Dragon Fruit', 'Hatchling Care Guide']),
    (ARRAY['Mystery at the Library', 'The Lost Book', 'Hidden Pages']),
    (ARRAY['Science of Flight', 'Bird Watching 101', 'Wings and Feathers']),
    (ARRAY['Database Internals', 'Index Structures', 'B-Tree Deep Dive']),
    (ARRAY['The Dragon Chronicles', 'Rise of the Phoenix', 'Ancient Legends']);

CREATE INDEX idx_books ON books
USING bm25 (id, all_titles)
WITH (
    key_field = 'id',
    text_fields = '{
        "all_titles": {
            "fast": true,
            "record": "position",
            "tokenizer": {
                "type": "icu"
            }
        },
        "all_titles_ngram": {
            "column": "all_titles",
            "fast": true,
            "record": "position",
            "tokenizer": {
                "type": "ngram",
                "min_gram": 4,
                "max_gram": 4,
                "prefix_only": false
            }
        }
    }'
);

-- Test 1: Single-word ngram match with conjunction_mode
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon', conjunction_mode => true)
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon', conjunction_mode => true)
ORDER BY id;

-- Test 2: Multi-word ngram match with conjunction_mode (13 Must clauses)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon Hatchling', conjunction_mode => true)
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon Hatchling', conjunction_mode => true)
ORDER BY id;

-- Test 3: disjunction_max combining boosted ICU match with ngram conjunction
-- Matches the pattern from the reported issue
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.disjunction_max(
    disjuncts => ARRAY[
        paradedb.boost(50, paradedb.match('all_titles', 'Dragon', prefix => true, conjunction_mode => true)),
        paradedb.match('all_titles_ngram', 'Dragon', conjunction_mode => true)
    ]
)
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.disjunction_max(
    disjuncts => ARRAY[
        paradedb.boost(50, paradedb.match('all_titles', 'Dragon', prefix => true, conjunction_mode => true)),
        paradedb.match('all_titles_ngram', 'Dragon', conjunction_mode => true)
    ]
)
ORDER BY id;

-- Test 4: Short query (fewer than min_gram characters — no tokens)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'SQL', conjunction_mode => true)
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'SQL', conjunction_mode => true)
ORDER BY id;

-- Test 5: Exact min_gram length query (single ngram token)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Fire', conjunction_mode => true)
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Fire', conjunction_mode => true)
ORDER BY id;

-- Test 6: JSON-based disjunction_max via ::jsonb path
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ '{"disjunction_max":{"disjuncts":[{"boost":{"factor":50,"query":{"match":{"field":"all_titles","value":"Dragon","prefix":true,"conjunction_mode":true}}}},{"match":{"field":"all_titles_ngram","value":"Dragon","prefix":false,"conjunction_mode":true}}]}}'::jsonb
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ '{"disjunction_max":{"disjuncts":[{"boost":{"factor":50,"query":{"match":{"field":"all_titles","value":"Dragon","prefix":true,"conjunction_mode":true}}}},{"match":{"field":"all_titles_ngram","value":"Dragon","prefix":false,"conjunction_mode":true}}]}}'::jsonb
ORDER BY id;

-- Test 7: Ngram match without conjunction_mode (disjunction baseline)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon')
ORDER BY id;

SELECT id, all_titles FROM books
WHERE id @@@ paradedb.match('all_titles_ngram', 'Dragon')
ORDER BY id;

DROP TABLE books;
