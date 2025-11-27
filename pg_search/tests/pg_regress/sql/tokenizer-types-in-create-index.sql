DROP TABLE IF EXISTS tok_in_ci;

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE tok_in_ci
(
    id serial8 not null primary key,
    t  text
);

INSERT INTO tok_in_ci (t)
VALUES ('this is a test');

CREATE INDEX idxtok_in_ci ON tok_in_ci USING bm25
    (
     id,
     t,
     (t::pdb.chinese_compatible('alias=chinese_compatible')),
     (t::pdb.literal('alias=literal')),
     (t::pdb.jieba('alias=jieba')),
     (t::pdb.lindera(chinese, 'alias=lindera_chinese')),
     (t::pdb.lindera(japanese, 'alias=lindera_japanese')),
     (t::pdb.lindera(korean, 'alias=lindera_korean')),
     (t::pdb.ngram(3, 5, 'alias=ngram')),
     (t::pdb.regex_pattern('is|a', 'alias=regex')),
     (t::pdb.simple('alias=simple')),
     (t::pdb.simple('stemmer=english', 'alias=stemmed')),
     (t::pdb.whitespace('alias=whitespace')),
     (t::pdb.source_code('alias=source_code'))
        )
    WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxtok_in_ci') ORDER BY name;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.chinese_compatible('alias=chinese_compatible')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.whitespace('alias=whitespace')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('stemmer=english', 'alias=stemmed')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('alias=simple')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.regex_pattern('is|a', 'alias=regex')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.ngram(3, 5, 'alias=ngram')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(korean, 'alias=lindera_korean')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(japanese, 'alias=lindera_japanese')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(chinese, 'alias=lindera_chinese')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.jieba('alias=jieba')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.literal('alias=literal')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.source_code('alias=source_code')) @@@ 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.chinese_compatible('alias=chinese_compatible')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.whitespace('alias=whitespace')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('stemmer=english', 'alias=stemmed')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('alias=simple')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.regex_pattern('is|a', 'alias=regex')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.ngram(3, 5, 'alias=ngram')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(korean, 'alias=lindera_korean')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(japanese, 'alias=lindera_japanese')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(chinese, 'alias=lindera_chinese')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.jieba('alias=jieba')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.literal('alias=literal')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.source_code('alias=source_code')) &&& 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.chinese_compatible('alias=chinese_compatible')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.whitespace('alias=whitespace')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('stemmer=english', 'alias=stemmed')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('alias=simple')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.regex_pattern('is|a', 'alias=regex')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.ngram(3, 5, 'alias=ngram')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(korean, 'alias=lindera_korean')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(japanese, 'alias=lindera_japanese')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(chinese, 'alias=lindera_chinese')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.jieba('alias=jieba')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.literal('alias=literal')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.source_code('alias=source_code')) ||| 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.chinese_compatible('alias=chinese_compatible')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.whitespace('alias=whitespace')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('stemmer=english', 'alias=stemmed')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.simple('alias=simple')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.regex_pattern('is|a', 'alias=regex')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.ngram(3, 5, 'alias=ngram')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(korean, 'alias=lindera_korean')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(japanese, 'alias=lindera_japanese')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.lindera(chinese, 'alias=lindera_chinese')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.jieba('alias=jieba')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.literal('alias=literal')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::pdb.source_code('alias=source_code')) === 'test';

RESET paradedb.enable_aggregate_custom_scan;
