CREATE EXTENSION IF NOT EXISTS pg_search;

SELECT 'Running Shoes.  olé'::pdb.simple::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.whitespace::text[];
SELECT 'Running Shoes.  olé'::pdb.whitespace('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.whitespace('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.literal::text[];
SELECT 'Running Shoes.  olé'::pdb.literal('alias=foo')::text[];  -- only option supported for exact
SELECT 'Running Shoes.  olé'::pdb.literal('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.chinese_compatible::text[];
SELECT 'Running Shoes.  olé'::pdb.chinese_compatible('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.chinese_compatible('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.lindera::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=chinese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=japanese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=korean')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera(chinese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(chinese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(japanese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(japanese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(korean, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(korean, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.jieba::text[];
SELECT 'Running Shoes.  olé'::pdb.jieba('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.jieba('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.ngram::text[]; -- error, needs min/max
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3)::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram('min=2', 'max=3', 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram('min=2', 'max=3', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Quick Fox'::pdb.edge_ngram(2, 5)::text[];
SELECT 'Quick Fox'::pdb.edge_ngram(1, 2)::text[];
SELECT 'Quick Fox'::pdb.edge_ngram(2, 5, 'lowercase=false')::text[];
SELECT 'Quick-Fox'::pdb.edge_ngram(2, 5, 'token_chars=letter,digit,punctuation')::text[];
SELECT 'Quick Fox'::pdb.edge_ngram('min=2', 'max=5')::text[];

-- End-to-end: create index, insert, search
DROP TABLE IF EXISTS edge_ngram_e2e;
CREATE TABLE edge_ngram_e2e (id serial8 NOT NULL PRIMARY KEY, name text);
INSERT INTO edge_ngram_e2e (name) VALUES ('PostgreSQL'), ('ParadeDB'), ('Paragraph');
CREATE INDEX idx_edge_ngram_e2e ON edge_ngram_e2e USING bm25 (id, (name::pdb.edge_ngram(2, 10))) WITH (key_field = 'id');
SELECT name FROM edge_ngram_e2e WHERE name @@@ 'par' ORDER BY name;
DROP TABLE edge_ngram_e2e;

SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=arabic')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=danish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=dutch')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=english')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=finnish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=french')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=german')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=greek')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=hungarian')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=italian')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=norwegian')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=polish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=portuguese')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=romanian')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=russian')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=spanish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=swedish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=tamil')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=turkish')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=foo')::text[]; -- error
SELECT 'Running Shoes.  olé'::pdb.simple('stemmer=english', 'lowercase=false', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.regex_pattern::text[]; -- error, needs a regular expression
SELECT 'Running Shoes.  olé'::pdb.regex_pattern('ing|oes')::text[];
SELECT 'Running Shoes.  olé'::pdb.regex_pattern('ing|oes', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.source_code::text[];
SELECT 'Running Shoes.  olé'::pdb.source_code('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.source_code('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.literal_normalized::text[];
SELECT 'Running Shoes.  olé'::pdb.literal_normalized('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.literal_normalized('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

-- Invalid configurations
SELECT 'Running Shoes.  olé'::pdb.simple('stemmmer=english')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('min=english')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('ascii_folding=foo')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('ascii_folding=f')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple(ascii_folding)::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('remove_short=0')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('remove_long=0')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 'invalid')::text[];

DROP TABLE IF EXISTS tokenizer_typmod_display;

CREATE TABLE tokenizer_typmod_display (
    id serial8 NOT NULL PRIMARY KEY,
    description text
);

CREATE INDEX idxtokenizer_typmod_display ON tokenizer_typmod_display USING bm25
    (
        id,
        description,
        (description::pdb.chinese_compatible('alias=chinese_compatible')),
        (description::pdb.literal('alias=literal')),
        (description::pdb.jieba('alias=jieba')),
        (description::pdb.lindera(chinese, 'alias=lindera_chinese')),
        (description::pdb.lindera(japanese, 'alias=lindera_japanese')),
        (description::pdb.lindera(korean, 'alias=lindera_korean')),
        (description::pdb.ngram(3, 5, 'alias=ngram')),
        (description::pdb.edge_ngram(2, 5, 'alias=edge_ngram')),
        (description::pdb.regex_pattern('is|a', 'alias=regex')),
        (description::pdb.simple('alias=simple')),
        (description::pdb.whitespace('alias=whitespace')),
        (description::pdb.source_code('alias=source_code')),
        (description::pdb.literal_normalized('alias=literal_normalized'))
    )
    WITH (key_field = 'id');

SELECT indexdef from pg_indexes where indexname = 'idxtokenizer_typmod_display';
DROP TABLE tokenizer_typmod_display;
