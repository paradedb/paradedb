-- Tests the `alyze` tokenizer (UAX #29 word segmentation via the alyze crate)
\i common/common_setup.sql

-- Inline tokenization. The default lowercases and keeps only word-like spans,
-- dropping punctuation and whitespace.
SELECT 'Hello, world! 123'::pdb.alyze::text[];

-- Contractions stay together; CJK ideographs split per-character; emoji are kept.
SELECT 'won''t 中文 👍'::pdb.alyze::text[];

-- With word_like=false, every UAX #29 segment is emitted, including punctuation
-- and whitespace.
SELECT 'a, b'::pdb.alyze('word_like=false')::text[];

-- Standard filters still apply on top of the tokenizer.
SELECT 'The Quick Brown Foxes'::pdb.alyze('stemmer=English')::text[];

-- The tokenizer is usable in a BM25 index.
DROP TABLE IF EXISTS alyze_items;
CREATE TABLE alyze_items(
    id serial8 not null primary key,
    description text
);
INSERT INTO alyze_items(description) VALUES
    ('Running shoes for athletes'),
    ('A quick brown fox'),
    ('Database indexing strategies');
CREATE INDEX idxalyze ON alyze_items USING bm25 (id, (description::pdb.alyze)) WITH (key_field = 'id');

SELECT id, description FROM alyze_items WHERE description @@@ 'running' ORDER BY id;
SELECT id, description FROM alyze_items WHERE description @@@ 'quick' ORDER BY id;

DROP TABLE alyze_items;
