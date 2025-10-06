DROP TABLE IF EXISTS all_types CASCADE;

--
-- since tokenizers are first-class types, they can be used
-- as column types in a table
--
CREATE TABLE all_types
(
    id                     serial8 not null primary key,
    text_col               text,
    varchar_col            varchar,
    chinese_compatible_col paradedb.chinese_compatible,
    exact_col              paradedb.exact,
    jieba_col              paradedb.jieba,
    lindera_chinese_col    paradedb.lindera(chinese),
    lindera_japanese_col   paradedb.lindera(japanese),
    lindera_korean_col     paradedb.lindera(korean),
    ngram_col              paradedb.ngram(3, 5),
    regex_col              paradedb.regex('ll|o'),
    simple_col             paradedb.simple,
    stemmed_en_col         paradedb.stemmed(english),
    whitespace_col         paradedb.whitespace
);

INSERT INTO all_types(text_col, varchar_col, chinese_compatible_col, exact_col, jieba_col, lindera_chinese_col,
                      lindera_japanese_col, lindera_korean_col,
                      ngram_col, regex_col, simple_col, stemmed_en_col, whitespace_col)
VALUES ('hello world', 'hello world', 'hello world', 'hello world', 'hello world', 'hello world',
        'hello world', 'hello world', 'hello world', 'hello world', 'hello world', 'hello world',
        'hello world');
CREATE INDEX idxall_types ON all_types USING bm25 (id, text_col, varchar_col, chinese_compatible_col, exact_col,
                                                   jieba_col, lindera_chinese_col,
                                                   lindera_japanese_col, lindera_korean_col,
                                                   ngram_col, regex_col, simple_col, stemmed_en_col,
                                                   whitespace_col) WITH (key_field = 'id');

SELECT * FROM all_types WHERE text_col @@@ 'HELLO';
SELECT * FROM all_types WHERE text_col &&& 'HELLO';
SELECT * FROM all_types WHERE text_col ||| 'HELLO';
SELECT * FROM all_types WHERE text_col === 'HELLO';
SELECT * FROM all_types WHERE text_col === 'hello';

SELECT * FROM all_types WHERE varchar_col @@@ 'HELLO';
SELECT * FROM all_types WHERE varchar_col &&& 'HELLO';
SELECT * FROM all_types WHERE varchar_col ||| 'HELLO';
SELECT * FROM all_types WHERE varchar_col === 'HELLO';
SELECT * FROM all_types WHERE varchar_col === 'hello';

SELECT * FROM all_types WHERE chinese_compatible_col @@@ 'HELLO';
SELECT * FROM all_types WHERE chinese_compatible_col &&& 'HELLO';
SELECT * FROM all_types WHERE chinese_compatible_col ||| 'HELLO';
SELECT * FROM all_types WHERE chinese_compatible_col === 'HELLO';
SELECT * FROM all_types WHERE chinese_compatible_col === 'hello';

SELECT * FROM all_types WHERE exact_col @@@ 'HELLO';
SELECT * FROM all_types WHERE exact_col &&& 'HELLO';
SELECT * FROM all_types WHERE exact_col ||| 'HELLO';
SELECT * FROM all_types WHERE exact_col === 'HELLO';
SELECT * FROM all_types WHERE exact_col === 'hello';

SELECT * FROM all_types WHERE exact_col @@@ 'hello world';
SELECT * FROM all_types WHERE exact_col &&& 'hello world';
SELECT * FROM all_types WHERE exact_col ||| 'hello world';
SELECT * FROM all_types WHERE exact_col === 'hello world';

SELECT * FROM all_types WHERE jieba_col @@@ 'HELLO';
SELECT * FROM all_types WHERE jieba_col &&& 'HELLO';
SELECT * FROM all_types WHERE jieba_col ||| 'HELLO';
SELECT * FROM all_types WHERE jieba_col === 'HELLO';
SELECT * FROM all_types WHERE jieba_col === 'hello';

SELECT * FROM all_types WHERE lindera_chinese_col @@@ 'HELLO';
SELECT * FROM all_types WHERE lindera_chinese_col &&& 'HELLO';
SELECT * FROM all_types WHERE lindera_chinese_col ||| 'HELLO';
SELECT * FROM all_types WHERE lindera_chinese_col === 'HELLO';
SELECT * FROM all_types WHERE lindera_chinese_col === 'hello';

SELECT * FROM all_types WHERE lindera_japanese_col @@@ 'HELLO';
SELECT * FROM all_types WHERE lindera_japanese_col &&& 'HELLO';
SELECT * FROM all_types WHERE lindera_japanese_col ||| 'HELLO';
SELECT * FROM all_types WHERE lindera_japanese_col === 'HELLO';
SELECT * FROM all_types WHERE lindera_japanese_col === 'hello';

SELECT * FROM all_types WHERE lindera_korean_col @@@ 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col &&& 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col ||| 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col === 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col === 'hello';

SELECT * FROM all_types WHERE lindera_korean_col @@@ 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col &&& 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col ||| 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col === 'HELLO';
SELECT * FROM all_types WHERE lindera_korean_col === 'hello';

SELECT * FROM all_types WHERE ngram_col @@@ 'HELLO';
SELECT * FROM all_types WHERE ngram_col &&& 'HELLO';
SELECT * FROM all_types WHERE ngram_col ||| 'HELLO';
SELECT * FROM all_types WHERE ngram_col === 'HELLO';
SELECT * FROM all_types WHERE ngram_col === 'hello';

SELECT * FROM all_types WHERE regex_col @@@ 'HELLO';
SELECT * FROM all_types WHERE regex_col &&& 'HELLO';
SELECT * FROM all_types WHERE regex_col ||| 'HELLO';
SELECT * FROM all_types WHERE regex_col === 'HELLO';
SELECT * FROM all_types WHERE regex_col === 'hello';

SELECT * FROM all_types WHERE simple_col @@@ 'HELLO';
SELECT * FROM all_types WHERE simple_col &&& 'HELLO';
SELECT * FROM all_types WHERE simple_col ||| 'HELLO';
SELECT * FROM all_types WHERE simple_col === 'HELLO';
SELECT * FROM all_types WHERE simple_col === 'hello';

SELECT * FROM all_types WHERE stemmed_en_col @@@ 'HELLO';
SELECT * FROM all_types WHERE stemmed_en_col &&& 'HELLO';
SELECT * FROM all_types WHERE stemmed_en_col ||| 'HELLO';
SELECT * FROM all_types WHERE stemmed_en_col === 'HELLO';
SELECT * FROM all_types WHERE stemmed_en_col === 'hello';

SELECT * FROM all_types WHERE whitespace_col @@@ 'HELLO';
SELECT * FROM all_types WHERE whitespace_col &&& 'HELLO';
SELECT * FROM all_types WHERE whitespace_col ||| 'HELLO';
SELECT * FROM all_types WHERE whitespace_col === 'HELLO';
SELECT * FROM all_types WHERE whitespace_col === 'hello';



