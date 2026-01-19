\i common/common_setup.sql

-- Test table for related_terms function
CREATE TABLE related_terms_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price INTEGER
);

INSERT INTO related_terms_test (description, category, price) VALUES
    ('running shoes for athletes', 'footwear', 100),
    ('comfortable running sneakers', 'footwear', 120),
    ('hiking boots for outdoor adventures', 'footwear', 150),
    ('casual shoes for everyday wear', 'footwear', 80),
    ('athletic running gear', 'apparel', 50),
    ('running shorts and tops', 'apparel', 40);

CREATE INDEX ON related_terms_test USING bm25 (id, description, category, price) WITH (key_field = 'id');

-- Basic test: find terms related to 'shoes'
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass
) ORDER BY weight DESC, field, term;

-- Test with specific fields
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description']
) ORDER BY weight DESC, field, term;

-- Test with max_query_terms limit
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'running',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    max_query_terms => 3
) ORDER BY weight DESC, field, term;

-- Test min_term_frequency filter (term must appear at least n times in matching docs)
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'running',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    min_term_frequency => 2
) ORDER BY weight DESC, field, term;

-- Test min_doc_frequency filter (term must appear in at least n docs)
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    min_doc_frequency => 2
) ORDER BY weight DESC, field, term;

-- Test max_doc_frequency filter (term must appear in at most n docs)
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'running',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    max_doc_frequency => 2
) ORDER BY weight DESC, field, term;

-- Test min_word_length filter
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    min_word_length => 6
) ORDER BY weight DESC, field, term;

-- Test max_word_length filter  
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description'],
    max_word_length => 5
) ORDER BY weight DESC, field, term;

-- Test with category field
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'footwear',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['category']
) ORDER BY weight DESC, field, term;

-- Test with multiple fields (demonstrates per-field DF calculation)
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'footwear',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description', 'category']
) ORDER BY weight DESC, field, term;

-- Test: query_term should be excluded from results
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'running',
    relation => 'related_terms_test'::regclass,
    fields => ARRAY['description']
) WHERE term = 'running';

-- Test: no matching documents returns empty result
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'nonexistentterm',
    relation => 'related_terms_test'::regclass
) ORDER BY weight DESC;

-- Test: error when no BM25 index exists
CREATE TABLE no_index_table (id SERIAL PRIMARY KEY, name TEXT);
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'test',
    relation => 'no_index_table'::regclass
);
DROP TABLE no_index_table;

-- Test: NULL fields uses all indexed non-JSON fields
SELECT field, term, weight FROM pdb.related_terms(
    query_term => 'shoes',
    relation => 'related_terms_test'::regclass,
    fields => NULL
) ORDER BY weight DESC, field, term LIMIT 10;

DROP TABLE related_terms_test;
