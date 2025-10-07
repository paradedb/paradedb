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
    chinese_compatible_col pdb.chinese_compatible,
    exact_col              pdb.exact,
    jieba_col              pdb.jieba,
    lindera_chinese_col    pdb.lindera(chinese),
    lindera_japanese_col   pdb.lindera(japanese),
    lindera_korean_col     pdb.lindera(korean),
    ngram_col              pdb.ngram(3, 5),
    regex_col              pdb.regex('ll|o'),
    simple_col             pdb.simple,
    stemmed_en_col         pdb.simple('stemmer=english'),
    whitespace_col         pdb.whitespace,
    source_code_col        pdb.source_code
);

INSERT INTO all_types(text_col, varchar_col, chinese_compatible_col, exact_col, jieba_col, lindera_chinese_col,
                      lindera_japanese_col, lindera_korean_col,
                      ngram_col, regex_col, simple_col, stemmed_en_col, whitespace_col, source_code_col)
VALUES ('hello world', 'hello world', 'hello world', 'hello world', 'hello world', 'hello world',
        'hello world', 'hello world', 'hello world', 'hello world', 'hello world', 'hello world',
        'hello world', 'hello world');
CREATE INDEX idxall_types ON all_types USING bm25 (id, text_col, varchar_col, chinese_compatible_col, exact_col,
                                                   jieba_col, lindera_chinese_col,
                                                   lindera_japanese_col, lindera_korean_col,
                                                   ngram_col, regex_col, simple_col, stemmed_en_col,
                                                   whitespace_col, source_code_col) WITH (key_field = 'id');

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

SELECT * FROM all_types WHERE source_code_col @@@ 'HELLO';
SELECT * FROM all_types WHERE source_code_col &&& 'HELLO';
SELECT * FROM all_types WHERE source_code_col ||| 'HELLO';
SELECT * FROM all_types WHERE source_code_col === 'HELLO';
SELECT * FROM all_types WHERE source_code_col === 'hello';
