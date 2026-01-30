-- Test multiple stemmers per field
-- This tests the feature allowing multiple language stemmers for multilingual content

-- Test v2 typmod syntax: single stemmer (backwards compatibility)
SELECT 'running shoes'::pdb.simple('stemmer=English')::text[];

-- Test v2 typmod syntax: multiple stemmers (comma-separated)
SELECT 'developers depuis'::pdb.simple('stemmer=English,French')::text[];

-- Test v2 typmod syntax: compare with and without stemmer
SELECT
  'running developers'::pdb.simple::text[],
  'running developers'::pdb.simple('stemmer=English')::text[];

-- Test French stemmer alone
SELECT 'courant rapidement'::pdb.simple('stemmer=French')::text[];

-- Test multiple stemmers with French and English
SELECT 'running depuis developers toujours'::pdb.simple('stemmer=English,French')::text[];

-- Test v1 paradedb.tokenizer() with stemmers parameter (array form)
SELECT * FROM paradedb.tokenize(
    paradedb.tokenizer('default', stemmers => ARRAY['English', 'French']),
    'running depuis developers toujours'
);

-- Test v1 paradedb.tokenizer() with single stemmer parameter (backwards compatible)
SELECT * FROM paradedb.tokenize(
    paradedb.tokenizer('default', stemmer => 'English'),
    'running shoes developers'
);

-- Clean up
DROP TABLE IF EXISTS test_multi_stemmer;

-- Create test table
CREATE TABLE test_multi_stemmer (
    id SERIAL PRIMARY KEY,
    content TEXT
);

-- Insert test data with English and French content (multilingual CRM scenario)
-- Using words where search term differs from indexed word (not just prefix match)
INSERT INTO test_multi_stemmer (content) VALUES
    ('The team is studying the new framework'),          -- "studying" stems to "studi"
    ('Several flies were flying around'),                -- "flying" stems to "fli"  
    ('Il est courant de voir cela'),                     -- French: "courant" stems to "cour"
    ('Elle mange rapidement'),                           -- French: "mange" stems to "mang"
    ('Full-Stack Developer studying French'),            -- Mixed English/French
    ('Les études sont importantes');                     -- French: "études" 

-- Create index with multiple stemmers (English and French)
CREATE INDEX idx_multi_stemmer_bm25 ON test_multi_stemmer
    USING bm25 (id, content)
    WITH (
    key_field = 'id',
    text_fields ='{
        "content": {"tokenizer": {"type": "default", "stemmer": ["English", "French"]}}
    }'
);

-- Test 1: Search for "studies" - should match "studying" (both stem to "studi")
-- Note: "studies" is NOT a substring of "studying"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'studies' ORDER BY id;

SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'studies' ORDER BY id;

-- Test 2: Search for "flies" - should match "flying" (both stem to "fli")
-- Note: "flies" is NOT a substring of "flying"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'flies' ORDER BY id;

SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'flies' ORDER BY id;

-- Test 3: Search for French "courir" - should match "courant" (both stem to "cour")
-- Note: "courir" is NOT a substring of "courant"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'courir' ORDER BY id;

SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'courir' ORDER BY id;

-- Test 4: Search for French "manger" - should match "mange" (both stem to "mang")
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'manger' ORDER BY id;

SELECT id, content FROM test_multi_stemmer WHERE content @@@ 'manger' ORDER BY id;

-- Clean up
DROP TABLE test_multi_stemmer;

-- Test single stemmer still works (backwards compatibility)
DROP TABLE IF EXISTS test_single_stemmer;

CREATE TABLE test_single_stemmer (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO test_single_stemmer (content) VALUES
    ('running quickly jumping');

-- Single stemmer as string (backwards compatible)
CREATE INDEX idx_single_stemmer_bm25 ON test_single_stemmer
    USING bm25 (id, content)
    WITH (
    key_field = 'id',
    text_fields ='{
        "content": {"tokenizer": {"type": "default", "stemmer": "English"}}
    }'
);

-- Test: Search for "run" (stemmed from "running") - should return 1 row
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_single_stemmer WHERE content @@@ 'run';

SELECT id, content FROM test_single_stemmer WHERE content @@@ 'run';

-- Test: Search for "jump" (stemmed from "jumping") - should return 1 row
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_single_stemmer WHERE content @@@ 'jump';

SELECT id, content FROM test_single_stemmer WHERE content @@@ 'jump';

-- Clean up
DROP TABLE test_single_stemmer;

-- Test three languages: English, French, Spanish
-- Using word pairs where search term differs from indexed word
DROP TABLE IF EXISTS test_three_stemmers;

CREATE TABLE test_three_stemmers (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO test_three_stemmers (content) VALUES
    ('team is studying hard'),        -- English: "studying" stems to "studi"
    ('il est courant ici'),           -- French: "courant" stems to "cour"
    ('está corriendo rápido');        -- Spanish: "corriendo" stems to "corr"

CREATE INDEX idx_three_stemmers_bm25 ON test_three_stemmers
    USING bm25 (id, content)
    WITH (
    key_field = 'id',
    text_fields ='{
        "content": {"tokenizer": {"type": "default", "stemmer": ["English", "French", "Spanish"]}}
    }'
);

-- Test: Search for "studies" - should match "studying" (both stem to "studi")
-- Note: "studies" is NOT a substring of "studying"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_three_stemmers WHERE content @@@ 'studies' ORDER BY id;

SELECT id, content FROM test_three_stemmers WHERE content @@@ 'studies' ORDER BY id;

-- Test: Search for French "courir" - should match "courant" (both stem to "cour")
-- Note: "courir" is NOT a substring of "courant"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_three_stemmers WHERE content @@@ 'courir' ORDER BY id;

SELECT id, content FROM test_three_stemmers WHERE content @@@ 'courir' ORDER BY id;

-- Test: Search for Spanish "correr" - should match "corriendo" (both stem to "corr")
-- Note: "correr" is NOT a substring of "corriendo"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM test_three_stemmers WHERE content @@@ 'correr' ORDER BY id;

SELECT id, content FROM test_three_stemmers WHERE content @@@ 'correr' ORDER BY id;

-- Clean up
DROP TABLE test_three_stemmers;
