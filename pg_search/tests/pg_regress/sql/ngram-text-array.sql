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
USING paradedb (id, (all_titles::pdb.icu('columnar=true')), (all_titles::pdb.ngram(4, 4, 'prefix_only=false', 'alias=all_titles_ngram', 'columnar=true')));

-- Test 1: Single-word ngram match with conjunction_mode
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Dragon'
ORDER BY id;

SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Dragon'
ORDER BY id;

-- Test 2: Multi-word ngram match with conjunction_mode (13 Must clauses)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Dragon Hatchling'
ORDER BY id;

SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Dragon Hatchling'
ORDER BY id;

-- Test 4: Short query (fewer than min_gram characters — no tokens)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'SQL'
ORDER BY id;

SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'SQL'
ORDER BY id;

-- Test 5: Exact min_gram length query (single ngram token)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Fire'
ORDER BY id;

SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') &&& 'Fire'
ORDER BY id;

-- Test 7: Ngram match without conjunction_mode (disjunction baseline)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') ||| 'Dragon'
ORDER BY id;

SELECT id, all_titles FROM books
WHERE all_titles::pdb.alias('all_titles_ngram') ||| 'Dragon'
ORDER BY id;

DROP TABLE books;
