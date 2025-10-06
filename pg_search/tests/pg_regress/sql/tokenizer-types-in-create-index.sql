DROP TABLE IF EXISTS tok_in_ci;
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
     (t::paradedb.chinese_compatible('alias=chinese_compatible')),
     (t::paradedb.exact('alias=exact')),
     (t::paradedb.jieba('alias=jieba')),
     (t::paradedb.lindera(chinese, 'alias=lindera_chinese')),
     (t::paradedb.lindera(japanese, 'alias=lindera_japanese')),
     (t::paradedb.lindera(korean, 'alias=lindera_korean')),
     (t::paradedb.ngram(3, 5, 'alias=ngram')),
     (t::paradedb.regex('is|a', 'alias=regex')),
     (t::paradedb.simple('alias=simple')),
     (t::paradedb.stemmed(english, 'alias=stemmed')),
     (t::paradedb.whitespace('alias=whitespace'))
        )
    WITH (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=chinese_compatible')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.whitespace('alias=whitespace')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.stemmed(english, 'alias=stemmed')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.simple('alias=simple')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.regex('is|a', 'alias=regex')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.ngram(3, 5, 'alias=ngram')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(korean, 'alias=lindera_korean')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(japanese, 'alias=lindera_japanese')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(chinese, 'alias=lindera_chinese')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.jieba('alias=jieba')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.exact('alias=exact')) @@@ 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=simple')) @@@ 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=chinese_compatible')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.whitespace('alias=whitespace')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.stemmed(english, 'alias=stemmed')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.simple('alias=simple')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.regex('is|a', 'alias=regex')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.ngram(3, 5, 'alias=ngram')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(korean, 'alias=lindera_korean')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(japanese, 'alias=lindera_japanese')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(chinese, 'alias=lindera_chinese')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.jieba('alias=jieba')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.exact('alias=exact')) &&& 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=simple')) &&& 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=chinese_compatible')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.whitespace('alias=whitespace')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.stemmed(english, 'alias=stemmed')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.simple('alias=simple')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.regex('is|a', 'alias=regex')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.ngram(3, 5, 'alias=ngram')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(korean, 'alias=lindera_korean')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(japanese, 'alias=lindera_japanese')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(chinese, 'alias=lindera_chinese')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.jieba('alias=jieba')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.exact('alias=exact')) ||| 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=simple')) ||| 'test';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE t === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=chinese_compatible')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.whitespace('alias=whitespace')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.stemmed(english, 'alias=stemmed')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.simple('alias=simple')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.regex('is|a', 'alias=regex')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.ngram(3, 5, 'alias=ngram')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(korean, 'alias=lindera_korean')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(japanese, 'alias=lindera_japanese')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.lindera(chinese, 'alias=lindera_chinese')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.jieba('alias=jieba')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.exact('alias=exact')) === 'test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM tok_in_ci WHERE (t::paradedb.chinese_compatible('alias=simple')) === 'test';

