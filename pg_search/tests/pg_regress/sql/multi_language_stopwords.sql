-- Test multiple stopwords languages per field
-- This tests the feature allowing multiple language stopwords filters

-- Clean up
DROP TABLE IF EXISTS test_multi_stopwords;

-- Create test table
CREATE TABLE test_multi_stopwords (
    id SERIAL PRIMARY KEY,
    content TEXT
);

-- Insert test data with English and French content
INSERT INTO test_multi_stopwords (content) VALUES
    ('the quick brown fox'),           -- "the" is English stopword
    ('le renard brun rapide'),          -- "le" is French stopword  
    ('and the lazy dog'),               -- "and", "the" are English stopwords
    ('et le chien paresseux'),          -- "et", "le" are French stopwords
    ('quick renard'),                   -- no stopwords
    ('fox chien'),                      -- no stopwords
    ('the quick fox le renard and et lazy paresseux');  -- Mixed English/French

-- Create index with multiple stopwords languages (English and French)
CREATE INDEX idx_multi_stopwords_bm25 ON test_multi_stopwords
    USING bm25 (id, content)
    WITH (
    key_field = 'id',
    text_fields ='{
        "content": {"tokenizer": {"type": "default", "stopwords_language": ["English", "French"]}}
    }'
);

-- Test 1: Search for English stopword "the" - should return 0 rows (filtered)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'the';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'the';

-- Test 2: Search for French stopword "le" - should return 0 rows (filtered)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'le';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'le';

-- Test 3: Search for non-stopword "quick" - should return rows
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'quick';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'quick';

-- Test 4: Search for non-stopword "renard" - should return rows  
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'renard';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'renard';

-- Test 5: Search for English stopword "and" - should return 0 rows (filtered)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'and';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'and';

-- Test 6: Search for French stopword "et" - should return 0 rows (filtered)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'et';

SELECT id, content FROM test_multi_stopwords WHERE content @@@ 'et';

-- Clean up
DROP TABLE test_multi_stopwords;

-- Test single language still works (backwards compatibility)
DROP TABLE IF EXISTS test_single_stopwords;

CREATE TABLE test_single_stopwords (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO test_single_stopwords (content) VALUES
    ('the quick brown fox and the lazy dog');

-- Single language as string (backwards compatible)
CREATE INDEX idx_single_stopwords_bm25 ON test_single_stopwords
    USING bm25 (id, content)
    WITH (
    key_field = 'id',
    text_fields ='{
        "content": {"tokenizer": {"type": "default", "stopwords_language": "English"}}
    }'
);

-- Test: Search for "the" (English stopword) - should return 0 rows
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_single_stopwords WHERE content @@@ 'the';

SELECT id, content FROM test_single_stopwords WHERE content @@@ 'the';

-- Test: Search for "quick" (not a stopword) - should return 1 row
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_single_stopwords WHERE content @@@ 'quick';

SELECT id, content FROM test_single_stopwords WHERE content @@@ 'quick';

-- Clean up
DROP TABLE test_single_stopwords;
