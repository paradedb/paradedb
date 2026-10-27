-- Chinese conversion and Jieba search_mode are available starting in 0.26.0.
CREATE TABLE tokenizer_recent_options (
    id INTEGER PRIMARY KEY,
    converted TEXT DEFAULT '繁體標題',
    ch_s2t TEXT DEFAULT '简体标题',
    ch_tw2s TEXT DEFAULT '鼠標里面的硅二極管壞了',
    ch_tw2sp TEXT DEFAULT '滑鼠',
    ch_s2tw TEXT DEFAULT '鼠标',
    ch_s2twp TEXT DEFAULT '鼠标',
    jb_t2s TEXT DEFAULT '繁體標題',
    jb_s2t TEXT DEFAULT '简体标题',
    jb_tw2s TEXT DEFAULT '鼠標里面的硅二極管壞了',
    jb_tw2sp TEXT DEFAULT '滑鼠',
    jb_s2tw TEXT DEFAULT '鼠标',
    jb_s2twp TEXT DEFAULT '鼠标',
    jieba_search TEXT DEFAULT '南京市长江大桥',
    jieba_no_search TEXT DEFAULT '南京市长江大桥',
    ch_filtered TEXT DEFAULT '繁體 CAFÉ',
    jb_filtered TEXT DEFAULT '繁體 CAFÉ'
);
INSERT INTO tokenizer_recent_options (id) VALUES (1);
CREATE INDEX tokenizer_recent_options_idx ON tokenizer_recent_options USING paradedb (
    id,
    (converted::pdb.chinese_compatible('chinese_convert=t2s')),
    (ch_s2t::pdb.chinese_compatible('chinese_convert=s2t')),
    (ch_tw2s::pdb.chinese_compatible('chinese_convert=tw2s')),
    (ch_tw2sp::pdb.chinese_compatible('chinese_convert=tw2sp')),
    (ch_s2tw::pdb.chinese_compatible('chinese_convert=s2tw')),
    (ch_s2twp::pdb.chinese_compatible('chinese_convert=s2twp')),
    (jb_t2s::pdb.jieba('chinese_convert=t2s')),
    (jb_s2t::pdb.jieba('chinese_convert=s2t')),
    (jb_tw2s::pdb.jieba('chinese_convert=tw2s')),
    (jb_tw2sp::pdb.jieba('chinese_convert=tw2sp')),
    (jb_s2tw::pdb.jieba('chinese_convert=s2tw')),
    (jb_s2twp::pdb.jieba('chinese_convert=s2twp')),
    (jieba_search::pdb.jieba('search_mode=true')),
    (jieba_no_search::pdb.jieba('search_mode=false')),
    (ch_filtered::pdb.chinese_compatible('chinese_convert=t2s', 'ascii_folding=true', 'lowercase=false')),
    (jb_filtered::pdb.jieba('chinese_convert=t2s', 'search_mode=false', 'ascii_folding=true', 'lowercase=false'))
);

CREATE VIEW tokenizer_recent_results AS
SELECT 'converted' AS query, ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('converted', '标') ORDER BY id
) AS ids
UNION ALL
SELECT 'unconverted', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('converted', '標') ORDER BY id
)
UNION ALL
SELECT 'jieba_search', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jieba_search', '南京') ORDER BY id
)
UNION ALL
SELECT 'jieba_no_search', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jieba_no_search', '南京') ORDER BY id
)
UNION ALL
SELECT 'jieba_full', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jieba_no_search', '南京市') ORDER BY id
)
UNION ALL
SELECT 'ch_s2t_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2t', '標') ORDER BY id
)
UNION ALL
SELECT 'ch_s2t_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2t', '标') ORDER BY id
)
UNION ALL
SELECT 'ch_tw2s_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_tw2s', '标') ORDER BY id
)
UNION ALL
SELECT 'ch_tw2s_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_tw2s', '標') ORDER BY id
)
UNION ALL
SELECT 'ch_tw2sp_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_tw2sp', '标') ORDER BY id
)
UNION ALL
SELECT 'ch_tw2sp_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_tw2sp', '滑') ORDER BY id
)
UNION ALL
SELECT 'ch_s2tw_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2tw', '標') ORDER BY id
)
UNION ALL
SELECT 'ch_s2tw_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2tw', '标') ORDER BY id
)
UNION ALL
SELECT 'ch_s2twp_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2twp', '滑') ORDER BY id
)
UNION ALL
SELECT 'ch_s2twp_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('ch_s2twp', '标') ORDER BY id
)
UNION ALL
SELECT 'jb_t2s_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_t2s', '标题') ORDER BY id
)
UNION ALL
SELECT 'jb_t2s_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_t2s', '標題') ORDER BY id
)
UNION ALL
SELECT 'jb_s2t_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2t', '標題') ORDER BY id
)
UNION ALL
SELECT 'jb_s2t_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2t', '标题') ORDER BY id
)
UNION ALL
SELECT 'jb_tw2s_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_tw2s', '鼠标') ORDER BY id
)
UNION ALL
SELECT 'jb_tw2s_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_tw2s', '鼠標') ORDER BY id
)
UNION ALL
SELECT 'jb_tw2sp_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_tw2sp', '鼠标') ORDER BY id
)
UNION ALL
SELECT 'jb_tw2sp_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_tw2sp', '滑鼠') ORDER BY id
)
UNION ALL
SELECT 'jb_s2tw_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2tw', '鼠標') ORDER BY id
)
UNION ALL
SELECT 'jb_s2tw_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2tw', '鼠标') ORDER BY id
)
UNION ALL
SELECT 'jb_s2twp_match', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2twp', '滑鼠') ORDER BY id
)
UNION ALL
SELECT 'jb_s2twp_miss', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE id @@@ paradedb.term('jb_s2twp', '鼠标') ORDER BY id
)
UNION ALL
SELECT 'ch_filtered_t2s', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE ch_filtered ||| '繁體' ORDER BY id
)
UNION ALL
SELECT 'ch_filtered_ascii_folding', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE ch_filtered ||| 'CAFÉ' ORDER BY id
)
UNION ALL
SELECT 'ch_filtered_t2s_rejects_original', ARRAY(
    SELECT id FROM tokenizer_recent_options
    WHERE id @@@ paradedb.term('ch_filtered', '繁體') ORDER BY id
)
UNION ALL
SELECT 'ch_filtered_folding_rejects_accent', ARRAY(
    SELECT id FROM tokenizer_recent_options
    WHERE id @@@ paradedb.term('ch_filtered', 'CAFÉ') ORDER BY id
)
UNION ALL
SELECT 'jieba_filtered_t2s', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE jb_filtered ||| '繁體' ORDER BY id
)
UNION ALL
SELECT 'jieba_filtered_ascii_folding', ARRAY(
    SELECT id FROM tokenizer_recent_options WHERE jb_filtered ||| 'CAFÉ' ORDER BY id
)
UNION ALL
SELECT 'jieba_filtered_t2s_rejects_original', ARRAY(
    SELECT id FROM tokenizer_recent_options
    WHERE id @@@ paradedb.term('jb_filtered', '繁體') ORDER BY id
)
UNION ALL
SELECT 'jieba_filtered_folding_rejects_accent', ARRAY(
    SELECT id FROM tokenizer_recent_options
    WHERE id @@@ paradedb.term('jb_filtered', 'CAFÉ') ORDER BY id
)
;

CREATE TABLE tokenizer_recent_before_upgrade AS SELECT * FROM tokenizer_recent_results;
CREATE TABLE tokenizer_recent_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('tokenizer_recent_options_idx');

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM tokenizer_recent_before_upgrade WHERE ids = ARRAY[1]) = 18,
        'conversion and Jieba positive cases must match before upgrade';
    ASSERT (SELECT count(*) FROM tokenizer_recent_before_upgrade WHERE ids = ARRAY[]::integer[]) = 17,
        'conversion and Jieba negative cases must not match before upgrade';
END;
$$;
