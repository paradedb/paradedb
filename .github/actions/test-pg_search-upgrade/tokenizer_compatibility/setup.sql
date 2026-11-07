-- This file runs against the prior extension release. Keep its SQL compatible
-- with every starting release in the upgrade matrix.
CREATE TABLE tokenizer_compatibility (
    id INTEGER PRIMARY KEY,
    about_me TEXT,
    bio TEXT,
    legacy_chinese TEXT,
    legacy_japanese TEXT,
    legacy_korean TEXT,
    current_chinese TEXT,
    current_japanese TEXT,
    current_korean TEXT,
    raw_text TEXT
);

INSERT INTO tokenizer_compatibility VALUES (
    1,
    'Hello World 🚀',
    'Hello World 🚀',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    'compat'
);

ALTER TABLE tokenizer_compatibility
    ADD COLUMN chinese_compatible TEXT DEFAULT '我们都有光明的前途',
    ADD COLUMN unicode_keep_emoji TEXT DEFAULT 'hello 🎂',
    ADD COLUMN unicode_remove_emoji TEXT DEFAULT 'hello 🎂',
    ADD COLUMN jieba TEXT DEFAULT '我们都有光明的前途',
    ADD COLUMN jieba_converted TEXT DEFAULT '繁體標題',
    ADD COLUMN icu TEXT DEFAULT 'محمد',
    ADD COLUMN edge_ngram TEXT DEFAULT 'ParadeDB',
    ADD COLUMN edge_digits TEXT DEFAULT 'abc12345',
    ADD COLUMN grams TEXT DEFAULT 'parade',
    ADD COLUMN grams_prefix TEXT DEFAULT 'parade',
    ADD COLUMN grams_positions TEXT DEFAULT 'parade',
    ADD COLUMN folded TEXT DEFAULT 'café',
    ADD COLUMN stemmed TEXT DEFAULT 'running',
    ADD COLUMN bounded TEXT DEFAULT 'a apple pineapple',
    ADD COLUMN simple_text TEXT DEFAULT 'hello world',
    ADD COLUMN whitespace_text TEXT DEFAULT 'hello world',
    ADD COLUMN source_code_text TEXT DEFAULT 'hello world',
    ADD COLUMN literal_text TEXT DEFAULT 'hello world',
    ADD COLUMN literal_normalized_text TEXT DEFAULT 'Hello World',
    ADD COLUMN japanese_nfkc TEXT DEFAULT 'ＡＢＣ１２３',
    ADD COLUMN japanese_reading TEXT DEFAULT '日本語',
    ADD COLUMN korean_reading TEXT DEFAULT '韓國';

CREATE INDEX tokenizer_compatibility_idx ON tokenizer_compatibility
USING paradedb (
    id,
    (about_me::pdb.unicode_words('remove_emojis=false')),
    (bio::pdb.unicode_words('lowercase=false')),
    legacy_chinese,
    legacy_japanese,
    legacy_korean,
    (current_chinese::pdb.lindera(chinese, 'alias=current_chinese')),
    (current_japanese::pdb.lindera(japanese, 'alias=current_japanese')),
    (current_korean::pdb.lindera(korean, 'alias=current_korean')),
    raw_text,
    (unicode_keep_emoji::pdb.unicode_words),
    (unicode_remove_emoji::pdb.unicode_words('remove_emojis=true')),
    (chinese_compatible::pdb.chinese_compatible),
    (jieba::pdb.jieba),
    (jieba_converted::pdb.jieba('chinese_convert=t2s')),
    (icu::pdb.icu),
    (edge_ngram::pdb.edge_ngram(2, 5)),
    (edge_digits::pdb.edge_ngram(2, 3, 'token_chars=digit')),
    (grams::pdb.ngram(2, 3, 'prefix_only=false')),
    (grams_prefix::pdb.ngram(2, 3, 'prefix_only=true')),
    (grams_positions::pdb.ngram(3, 3, 'positions=true')),
    (folded::pdb.simple('ascii_folding=true')),
    (stemmed::pdb.simple('stemmer=english')),
    (bounded::pdb.simple('remove_short=2', 'remove_long=6')),
    (simple_text::pdb.simple),
    (whitespace_text::pdb.whitespace),
    (source_code_text::pdb.source_code),
    (literal_text::pdb.literal),
    (literal_normalized_text::pdb.literal_normalized),
    (japanese_nfkc::pdb.lindera(japanese, 'nfkc=true')),
    (japanese_reading::pdb.lindera(japanese, 'reading_form=true')),
    (korean_reading::pdb.lindera(korean, 'reading_form=true'))
)
WITH (
    key_field = 'id',
    text_fields = '{
        "legacy_chinese": {"tokenizer": {"type": "chinese_lindera"}},
        "legacy_japanese": {"tokenizer": {"type": "japanese_lindera"}},
        "legacy_korean": {"tokenizer": {"type": "korean_lindera"}},
        "raw_text": {"tokenizer": {"type": "raw"}}
    }'
);

CREATE TABLE tokenizer_search_tokenizer_compatibility (
    id INTEGER PRIMARY KEY,
    autocomplete TEXT
);

INSERT INTO tokenizer_search_tokenizer_compatibility VALUES (1, 'hello world');

CREATE INDEX tokenizer_search_tokenizer_idx ON tokenizer_search_tokenizer_compatibility
USING paradedb (
    id,
    (autocomplete::pdb.ngram(1, 3, 'prefix_only=true'))
)
WITH (key_field = 'id', search_tokenizer = 'unicode_words');

CREATE TABLE tokenizer_filter_compatibility (
    id INTEGER PRIMARY KEY,
    stopword_text TEXT DEFAULT 'the fox',
    alphanumeric_text TEXT DEFAULT 'abc dé',
    custom_stopwords TEXT DEFAULT 'sentinel fox',
    keyword_text TEXT DEFAULT 'MiXeD text',
    combined_filters TEXT DEFAULT 'A LONGTOKEN CAt Café',
    legacy_trim_pair_trimmed TEXT DEFAULT ' hello ',
    legacy_trim_pair_untrimmed TEXT DEFAULT ' hello ',
    edge_letters TEXT DEFAULT 'ab 12-+',
    edge_whitespace TEXT DEFAULT 'ab 12-+',
    edge_punctuation TEXT DEFAULT 'ab 12-+',
    edge_symbol TEXT DEFAULT 'ab 12-+',
    normalizer_raw TEXT DEFAULT 'MiXeD',
    zh_keep_space TEXT DEFAULT '北京 新餐馆',
    zh_drop_space TEXT DEFAULT '北京 新餐馆',
    jp_keep_space TEXT DEFAULT '東京 カフェ',
    jp_drop_space TEXT DEFAULT '東京 カフェ',
    ko_keep_space TEXT DEFAULT '김민준 서울',
    ko_drop_space TEXT DEFAULT '김민준 서울',
    normalizer_lower TEXT DEFAULT 'MiXeD'
);
INSERT INTO tokenizer_filter_compatibility (id) VALUES (1);
CREATE INDEX tokenizer_filter_compatibility_idx ON tokenizer_filter_compatibility
USING paradedb (
    id,
    (stopword_text::pdb.simple('stopwords_language=english')),
    (alphanumeric_text::pdb.simple('alpha_num_only=true')),
    custom_stopwords,
    keyword_text,
    (combined_filters::pdb.simple('remove_short=2', 'remove_long=6', 'lowercase=false', 'ascii_folding=true')),
    (legacy_trim_pair_trimmed::pdb.jieba('trim=true')),
    (legacy_trim_pair_untrimmed::pdb.jieba('trim=false')),
    (edge_letters::pdb.edge_ngram(1, 1, 'token_chars=letter')),
    (edge_whitespace::pdb.edge_ngram(1, 1, 'token_chars=whitespace')),
    (edge_punctuation::pdb.edge_ngram(1, 1, 'token_chars=punctuation')),
    (edge_symbol::pdb.edge_ngram(1, 1, 'token_chars=symbol')),
    (normalizer_raw::pdb.literal_normalized('lowercase=false', 'normalizer=raw', 'columnar=true')),
    (zh_keep_space::pdb.lindera(chinese, 'keep_whitespace=true')),
    (zh_drop_space::pdb.lindera(chinese, 'keep_whitespace=false')),
    (jp_keep_space::pdb.lindera(japanese, 'keep_whitespace=true')),
    (jp_drop_space::pdb.lindera(japanese, 'keep_whitespace=false')),
    (ko_keep_space::pdb.lindera(korean, 'keep_whitespace=true')),
    (ko_drop_space::pdb.lindera(korean, 'keep_whitespace=false')),
    (normalizer_lower::pdb.literal_normalized('lowercase=false', 'normalizer=lowercase', 'columnar=true'))
) WITH (
    key_field = 'id',
    text_fields = '{
        "custom_stopwords": {"tokenizer": {"type": "default", "stopwords": ["sentinel"]}},
        "keyword_text": {"tokenizer": {"type": "keyword"}}
    }'
);

CREATE VIEW tokenizer_compatibility_results AS
SELECT 'unicode_field' AS query, ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE about_me ||| 'hello' ORDER BY id
) AS ids
UNION ALL
SELECT 'unicode_query_cast', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE about_me ||| 'hello'::pdb.unicode_words('remove_emojis=false') ORDER BY id
)
UNION ALL
SELECT 'unicode_lowercase_false', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE bio ||| 'Hello'::pdb.unicode_words('lowercase=false') ORDER BY id
)
UNION ALL
SELECT 'unicode_lowercase_false_uncast', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE bio ||| 'Hello' ORDER BY id
)
UNION ALL
SELECT 'legacy_chinese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_chinese ||| '北京' ORDER BY id
)
UNION ALL
SELECT 'legacy_japanese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_japanese ||| '東京' ORDER BY id
)
UNION ALL
SELECT 'legacy_korean', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_korean ||| '김민준' ORDER BY id
)
UNION ALL
SELECT 'current_chinese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_chinese ||| '北京' ORDER BY id
)
UNION ALL
SELECT 'current_japanese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_japanese ||| '東京' ORDER BY id
)
UNION ALL
SELECT 'current_korean', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_korean ||| '김민준' ORDER BY id
)
UNION ALL
SELECT 'raw', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE raw_text ||| 'compat' ORDER BY id
)
UNION ALL
SELECT 'unicode_keep_emoji', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE unicode_keep_emoji ||| '🎂'::pdb.literal ORDER BY id
)
UNION ALL
SELECT 'chinese_compatible', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE chinese_compatible ||| '光' ORDER BY id
)
UNION ALL
SELECT 'jieba', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE jieba ||| '光明' ORDER BY id
)
UNION ALL
SELECT 'jieba_converted', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE jieba_converted ||| '标题'::pdb.literal ORDER BY id
)
UNION ALL
SELECT 'icu', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE icu ||| 'محمد' ORDER BY id
)
UNION ALL
SELECT 'edge_ngram', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE edge_ngram ||| 'par' ORDER BY id
)
UNION ALL
SELECT 'edge_digits', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE edge_digits ||| '123' ORDER BY id
)
UNION ALL
SELECT 'ngram', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE grams ||| 'ara' ORDER BY id
)
UNION ALL
SELECT 'ngram_prefix', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE grams_prefix ||| 'par' ORDER BY id
)
UNION ALL
SELECT 'ngram_positions', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE grams_positions ### ARRAY['par', 'ara'] ORDER BY id
)
UNION ALL
SELECT 'ascii_folding', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE folded ||| 'cafe' ORDER BY id
)
UNION ALL
SELECT 'stemmer', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE stemmed ||| 'run' ORDER BY id
)
UNION ALL
SELECT 'length_filters', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE bounded ||| 'apple'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'simple', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE simple_text ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'whitespace', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE whitespace_text ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'source_code', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE source_code_text ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'literal', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE literal_text ||| 'hello world' ORDER BY id
)
UNION ALL
SELECT 'literal_normalized', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE literal_normalized_text ||| 'hello world' ORDER BY id
)
UNION ALL
SELECT 'japanese_nfkc', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE japanese_nfkc ||| 'abc'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'japanese_reading', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE japanese_reading ||| 'ニホンゴ'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'korean_reading', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE korean_reading ||| '한국'::pdb.simple ORDER BY id
);

CREATE VIEW tokenizer_compatibility_negative_results AS
SELECT 'unicode_remove_emoji' AS query, ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE unicode_remove_emoji ||| '🎂'::pdb.literal ORDER BY id
) AS ids
UNION ALL
SELECT 'literal_partial', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE id @@@ paradedb.term('literal_text', 'hello') ORDER BY id
)
UNION ALL
SELECT 'literal_normalized_case', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE id @@@ paradedb.term('literal_normalized_text', 'Hello World') ORDER BY id
)
UNION ALL
SELECT 'chinese_compatible', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE chinese_compatible ||| '猫' ORDER BY id
)
UNION ALL
SELECT 'jieba', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE jieba ||| '公安' ORDER BY id
)
UNION ALL
SELECT 'icu', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE icu ||| 'السوق' ORDER BY id
)
UNION ALL
SELECT 'edge_ngram', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE edge_ngram ||| 'zoo' ORDER BY id
)
UNION ALL
SELECT 'edge_digits', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE edge_digits ||| 'abc'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'ngram_prefix', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE grams_prefix ||| 'ara'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'ngram_positions_reversed', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE grams_positions ### ARRAY['ara', 'par'] ORDER BY id
)
UNION ALL
SELECT 'remove_short', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE bounded ||| 'a'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'remove_long', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE bounded ||| 'pineapple'::pdb.simple ORDER BY id
);

CREATE VIEW tokenizer_filter_results AS
SELECT 'stopwords_language' AS query, ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE stopword_text ||| 'fox' ORDER BY id
) AS ids
UNION ALL
SELECT 'alpha_num_only', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE alphanumeric_text ||| 'abc' ORDER BY id
)
UNION ALL
SELECT 'custom_stopwords', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE custom_stopwords ||| 'fox' ORDER BY id
)
UNION ALL
SELECT 'keyword', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('keyword_text', 'MiXeD text') ORDER BY id
)
UNION ALL
SELECT 'combined_case', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'CAt') ORDER BY id
)
UNION ALL
SELECT 'combined_folding', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'Cafe') ORDER BY id
)
UNION ALL
SELECT 'edge_letter', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_letters', 'a') ORDER BY id
)
UNION ALL
SELECT 'edge_whitespace', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_whitespace', ' ') ORDER BY id
)
UNION ALL
SELECT 'edge_punctuation', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_punctuation', '-') ORDER BY id
)
UNION ALL
SELECT 'edge_symbol', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_symbol', '+') ORDER BY id
)
UNION ALL
SELECT 'stopword_removed', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('stopword_text', 'the') ORDER BY id
)
UNION ALL
SELECT 'non_ascii_removed', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('alphanumeric_text', 'dé') ORDER BY id
)
UNION ALL
SELECT 'custom_stopword_removed', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('custom_stopwords', 'sentinel') ORDER BY id
)
UNION ALL
SELECT 'keyword_case', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('keyword_text', 'mixed text') ORDER BY id
)
UNION ALL
SELECT 'keyword_partial', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('keyword_text', 'MiXeD') ORDER BY id
)
UNION ALL
SELECT 'combined_short', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'A') ORDER BY id
)
UNION ALL
SELECT 'combined_long', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'LONGTOKEN') ORDER BY id
)
UNION ALL
SELECT 'combined_lowercase', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'cat') ORDER BY id
)
UNION ALL
SELECT 'combined_unfolded', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('combined_filters', 'Café') ORDER BY id
)
UNION ALL
SELECT 'legacy_trim_pair_trimmed_space', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility
    WHERE id @@@ paradedb.term('legacy_trim_pair_trimmed', ' ') ORDER BY id
)
UNION ALL
SELECT 'legacy_trim_pair_untrimmed_space', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility
    WHERE id @@@ paradedb.term('legacy_trim_pair_untrimmed', ' ') ORDER BY id
)
UNION ALL
SELECT 'edge_letter_rejects_digit', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_letters', '1') ORDER BY id
)
UNION ALL
SELECT 'edge_whitespace_rejects_letter', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_whitespace', 'a') ORDER BY id
)
UNION ALL
SELECT 'edge_punctuation_rejects_letter', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_punctuation', 'a') ORDER BY id
)
UNION ALL
SELECT 'edge_symbol_rejects_letter', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('edge_symbol', 'a') ORDER BY id
)
UNION ALL
SELECT 'zh_keep', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('zh_keep_space', ' ') ORDER BY id
)
UNION ALL
SELECT 'zh_drop', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('zh_drop_space', '北京') ORDER BY id
)
UNION ALL
SELECT 'zh_keep_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('zh_keep_space', 'unrelated') ORDER BY id
)
UNION ALL
SELECT 'zh_drop_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('zh_drop_space', ' ') ORDER BY id
)
UNION ALL
SELECT 'jp_keep', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('jp_keep_space', ' ') ORDER BY id
)
UNION ALL
SELECT 'jp_drop', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('jp_drop_space', '東京') ORDER BY id
)
UNION ALL
SELECT 'jp_keep_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('jp_keep_space', 'unrelated') ORDER BY id
)
UNION ALL
SELECT 'jp_drop_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('jp_drop_space', ' ') ORDER BY id
)
UNION ALL
SELECT 'ko_keep', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('ko_keep_space', ' ') ORDER BY id
)
UNION ALL
SELECT 'ko_drop', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('ko_drop_space', '김민준') ORDER BY id
)
UNION ALL
SELECT 'ko_keep_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('ko_keep_space', 'unrelated') ORDER BY id
)
UNION ALL
SELECT 'ko_drop_miss', ARRAY(
    SELECT id FROM tokenizer_filter_compatibility WHERE id @@@ paradedb.term('ko_drop_space', ' ') ORDER BY id
)
;
CREATE TABLE tokenizer_filter_results_before_upgrade AS SELECT * FROM tokenizer_filter_results;
CREATE TABLE tokenizer_filter_schema_before_upgrade AS
SELECT name, tokenizer, normalizer FROM paradedb.schema('tokenizer_filter_compatibility_idx');

CREATE VIEW tokenizer_normalizer_results AS
SELECT
    (SELECT pdb.agg('{"terms":{"field":"normalizer_raw"}}'::jsonb)
     FROM tokenizer_filter_compatibility WHERE id @@@ pdb.all()) AS raw_result,
    (SELECT pdb.agg('{"terms":{"field":"normalizer_lower"}}'::jsonb)
     FROM tokenizer_filter_compatibility WHERE id @@@ pdb.all()) AS lower_result;

CREATE TABLE tokenizer_compatibility_results_before_upgrade AS
SELECT * FROM tokenizer_compatibility_results;

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (
        SELECT count(*) FROM tokenizer_compatibility_results_before_upgrade
        WHERE ids = ARRAY[1]
    ) = 32, 'every field tokenizer compatibility case must match the original row before upgrade';
    ASSERT (SELECT count(*) FROM tokenizer_filter_results
            WHERE ids = ARRAY[1] AND query NOT LIKE 'legacy_trim_pair%') = 16,
        'filter positive cases must match before upgrade';
    ASSERT (SELECT count(*) FROM tokenizer_filter_results
            WHERE ids = ARRAY[]::integer[] AND query NOT LIKE 'legacy_trim_pair%') = 19,
        'filter negative cases must not match before upgrade';
    ASSERT (SELECT raw_result #>> '{buckets,0,key}' = 'MiXeD'
                   AND lower_result #>> '{buckets,0,key}' = 'mixed'
                   AND (raw_result #>> '{buckets,0,doc_count}')::integer = 1
                   AND (lower_result #>> '{buckets,0,doc_count}')::integer = 1
            FROM tokenizer_normalizer_results),
        'raw and lowercase normalizers must retain distinct fast-field values before upgrade';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility_negative_results WHERE ids <> ARRAY[]::integer[]
    ), 'negative tokenizer queries must not match before upgrade';
    ASSERT (SELECT ARRAY(
        SELECT id FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hel' ORDER BY id
    )) = ARRAY[1], 'search_tokenizer must match the prefix query before upgrade';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hello'
    ), 'search_tokenizer must reject a query longer than the indexed ngrams';
END;
$$;

CREATE TABLE tokenizer_compatibility_schema_before_upgrade AS
SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
FROM paradedb.schema('tokenizer_compatibility_idx');

-- Keep boundary and option-combination cases in a separate index so index-wide
-- search-tokenizer settings cannot hide field-tokenizer lookup regressions.
CREATE TABLE tokenizer_option_matrix (
    id INTEGER PRIMARY KEY,
    simple_text TEXT DEFAULT 'hello world',
    whitespace_text TEXT DEFAULT 'hello world',
    source_code_text TEXT DEFAULT 'HTTPRequest',
    literal_text TEXT DEFAULT 'Case Sensitive',
    chinese_ascii TEXT DEFAULT '北京 CAFÉ',
    jieba_ascii TEXT DEFAULT '南京市 CAFÉ',
    icu_hindi TEXT DEFAULT 'नमस्ते दुनिया',
    icu_ascii TEXT DEFAULT 'CAFÉ 東京',
    jp_nfkc_ws TEXT DEFAULT 'ＡＢＣ１２３ 東京',
    jp_reading_trim TEXT DEFAULT '日本語',
    ko_reading_trim TEXT DEFAULT '韓國',
    ngram_min TEXT DEFAULT 'ab',
    ngram_max TEXT DEFAULT 'parade',
    edge_bounds TEXT DEFAULT 'ParadeDB',
    edge_letter_digit TEXT DEFAULT 'ab 12-+',
    edge_space_punct TEXT DEFAULT 'ab -+',
    edge_symbol_digit TEXT DEFAULT '1+$'
);
INSERT INTO tokenizer_option_matrix (id) VALUES (1);

CREATE INDEX tokenizer_option_matrix_idx ON tokenizer_option_matrix USING paradedb (
    id,
    (simple_text::pdb.simple),
    (whitespace_text::pdb.whitespace),
    (source_code_text::pdb.source_code),
    (literal_text::pdb.literal),
    (chinese_ascii::pdb.chinese_compatible('ascii_folding=true')),
    (jieba_ascii::pdb.jieba('ascii_folding=true')),
    (icu_hindi::pdb.icu),
    (icu_ascii::pdb.icu('ascii_folding=true')),
    (jp_nfkc_ws::pdb.lindera(japanese, 'nfkc=true', 'keep_whitespace=true')),
    (jp_reading_trim::pdb.lindera(japanese, 'reading_form=true', 'trim=true')),
    (ko_reading_trim::pdb.lindera(korean, 'reading_form=true', 'trim=true')),
    (ngram_min::pdb.ngram(1, 2, 'prefix_only=false')),
    (ngram_max::pdb.ngram(2, 4, 'prefix_only=false')),
    (edge_bounds::pdb.edge_ngram(1, 5)),
    (edge_letter_digit::pdb.edge_ngram(1, 2, 'token_chars=letter,digit')),
    (edge_space_punct::pdb.edge_ngram(1, 2, 'token_chars=whitespace,punctuation')),
    (edge_symbol_digit::pdb.edge_ngram(1, 2, 'token_chars=symbol,digit'))
) WITH (key_field = 'id');

CREATE VIEW tokenizer_option_matrix_results AS
SELECT 'simple_positive' AS query, true AS expected_match, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE simple_text ||| 'hello' ORDER BY id
) AS ids
UNION ALL
SELECT 'simple_negative', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE simple_text ||| 'absent' ORDER BY id
)
UNION ALL
SELECT 'whitespace_positive', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE whitespace_text ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'whitespace_negative', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE whitespace_text ||| 'absent' ORDER BY id
)
UNION ALL
SELECT 'source_code_camel_case', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE source_code_text ||| 'request' ORDER BY id
)
UNION ALL
SELECT 'source_code_negative', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE source_code_text ||| 'absent' ORDER BY id
)
UNION ALL
SELECT 'literal_whole_value', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('literal_text', 'Case Sensitive') ORDER BY id
)
UNION ALL
SELECT 'literal_rejects_partial', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('literal_text', 'Case') ORDER BY id
)
UNION ALL
SELECT 'chinese_compatible_ascii_folding', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE chinese_ascii ||| 'cafe' ORDER BY id
)
UNION ALL
-- Jieba splits CAFÉ into "caf" and "é" before the folding filter runs.
SELECT 'jieba_ascii_folding', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('jieba_ascii', 'e') ORDER BY id
)
UNION ALL
SELECT 'jieba_ascii_rejects_accent', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('jieba_ascii', 'é') ORDER BY id
)
UNION ALL
SELECT 'icu_hindi_script', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE icu_hindi ||| 'नमस्ते' ORDER BY id
)
UNION ALL
SELECT 'icu_hindi_negative', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE icu_hindi ||| 'अनुपस्थित' ORDER BY id
)
UNION ALL
SELECT 'icu_ascii_folding', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE icu_ascii ||| 'cafe' ORDER BY id
)
UNION ALL
SELECT 'japanese_nfkc', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE jp_nfkc_ws ||| 'abc' ORDER BY id
)
UNION ALL
SELECT 'japanese_keep_whitespace', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('jp_nfkc_ws', ' ') ORDER BY id
)
UNION ALL
SELECT 'japanese_reading_with_trim', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE jp_reading_trim ||| 'ニホンゴ'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'korean_reading_with_trim', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix WHERE ko_reading_trim ||| '한국'::pdb.simple ORDER BY id
)
UNION ALL
SELECT 'ngram_min_boundary', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('ngram_min', 'a') ORDER BY id
)
UNION ALL
SELECT 'ngram_max_boundary', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('ngram_max', 'para') ORDER BY id
)
UNION ALL
SELECT 'ngram_rejects_over_max', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('ngram_max', 'parad') ORDER BY id
)
UNION ALL
SELECT 'edge_min_boundary', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_bounds', 'p') ORDER BY id
)
UNION ALL
SELECT 'edge_max_boundary', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_bounds', 'parad') ORDER BY id
)
UNION ALL
SELECT 'edge_rejects_over_max', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_bounds', 'parade') ORDER BY id
)
UNION ALL
SELECT 'edge_letter_digit_pair', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_letter_digit', '12') ORDER BY id
)
UNION ALL
SELECT 'edge_letter_digit_rejects_punctuation', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_letter_digit', '12-') ORDER BY id
)
UNION ALL
SELECT 'edge_whitespace_punctuation_pair', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_space_punct', ' -') ORDER BY id
)
UNION ALL
SELECT 'edge_whitespace_punctuation_rejects_symbol', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_space_punct', '-+') ORDER BY id
)
UNION ALL
SELECT 'edge_symbol_digit_pair', true, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_symbol_digit', '1+') ORDER BY id
)
UNION ALL
SELECT 'edge_symbol_digit_rejects_over_max', false, ARRAY(
    SELECT id FROM tokenizer_option_matrix
    WHERE id @@@ paradedb.term('edge_symbol_digit', '1+$') ORDER BY id
);

CREATE TABLE tokenizer_option_matrix_results_before_upgrade AS
SELECT * FROM tokenizer_option_matrix_results;
CREATE TABLE tokenizer_option_matrix_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('tokenizer_option_matrix_idx')
WHERE name NOT IN ('id', 'ctid');

SET plpgsql.check_asserts = on;
DO $$
DECLARE
    mismatches TEXT;
BEGIN
    SELECT string_agg(
        format('%s: expected_match=%s, ids=%s', query, expected_match, ids),
        '; ' ORDER BY query
    )
    INTO mismatches
    FROM tokenizer_option_matrix_results_before_upgrade
    WHERE (expected_match AND ids <> ARRAY[1])
        OR (NOT expected_match AND ids <> ARRAY[]::integer[]);

    ASSERT mismatches IS NULL,
        format(
            'tokenizer option, boundary, and negative-control cases must match before upgrade; mismatches: %s',
            COALESCE(mismatches, '<none>')
        );
END;
$$;
