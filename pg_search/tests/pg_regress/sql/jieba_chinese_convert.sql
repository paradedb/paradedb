-- Test Chinese Traditional/Simplified conversion with Jieba tokenizer
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Test 1: Basic T2S (Traditional to Simplified) tokenization
SELECT '繁體中文測試'::pdb.jieba('chinese_convert=t2s')::text[];

-- Test 2: Basic S2T (Simplified to Traditional) tokenization
SELECT '简体中文测试'::pdb.jieba('chinese_convert=s2t')::text[];

-- Test 3: Taiwan vocabulary conversion
SELECT '鼠標里面的硅二極管壞了'::pdb.jieba('chinese_convert=tw2s')::text[];

-- Test 4: Taiwan vocabulary conversion
SELECT '鼠标里面的硅二极管坏了'::pdb.jieba('chinese_convert=s2tw')::text[];

-- Test 5: Taiwan vocabulary conversion with idioms
SELECT '鼠標里面的硅二極管壞了'::pdb.jieba('chinese_convert=tw2sp')::text[];


-- Test 6: Create table and index with T2S
CREATE TABLE test_chinese_convert (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

-- Insert mixed Traditional and Simplified Chinese
INSERT INTO test_chinese_convert (title, content) VALUES
    ('繁體標題', '這是繁體中文的內容測試'),
    ('简体标题', '这是简体中文的内容测试'),
    ('運動鞋', '適合跑步和運動的鞋子'),
    ('运动鞋', '适合跑步和运动的鞋子'),
    ('電腦配件', '鼠標、鍵盤、顯示器'),
    ('电脑配件', '鼠标、键盘、显示器');

-- Create index with T2S conversion
CREATE INDEX test_chinese_bm25 ON test_chinese_convert
USING bm25 (id, (title::pdb.jieba('chinese_convert=t2s')), (content::pdb.jieba('chinese_convert=t2s')))
WITH (key_field='id');

-- Test 6: Query with Traditional Chinese (should match both Traditional and Simplified)
SELECT id, title FROM test_chinese_convert 
WHERE title ||| '標題'
ORDER BY id;

-- Test 7: Query with Simplified Chinese (should match both Traditional and Simplified)
SELECT id, title FROM test_chinese_convert 
WHERE title ||| '标题'
ORDER BY id;

-- Test 8: Query with Traditional term
SELECT id, title FROM test_chinese_convert 
WHERE title ||| '運動'
ORDER BY id;

-- Test 9: Query with Simplified term
SELECT id, title FROM test_chinese_convert 
WHERE title ||| '运动'
ORDER BY id;

-- Test 10: Content search with Traditional Chinese
SELECT id, title FROM test_chinese_convert 
WHERE content ||| '鼠標'
ORDER BY id;

-- Test 11: Content search with Simplified Chinese
SELECT id, title FROM test_chinese_convert 
WHERE content ||| '鼠标'
ORDER BY id;

-- Test 12: Test with S2T index
DROP INDEX test_chinese_bm25;
CREATE INDEX test_chinese_bm25 ON test_chinese_convert
USING bm25 (id, (title::pdb.jieba('chinese_convert=s2t')))
WITH (key_field='id');

SELECT id, title FROM test_chinese_convert 
WHERE title ||| '标题'
ORDER BY id;

-- Test 13: Test Taiwan vocabulary conversion
SELECT '鼠标'::pdb.jieba('chinese_convert=s2tw')::text[];  -- Should convert to 滑鼠
SELECT '硬盘'::pdb.jieba('chinese_convert=s2tw')::text[];  -- Should convert to 硬碟
SELECT '软件'::pdb.jieba('chinese_convert=s2tw')::text[];  -- Should convert to 軟體
SELECT '信息'::pdb.jieba('chinese_convert=s2tw')::text[];  -- Should convert to 資訊

-- Test 14: Test with filters
SELECT '繁體中文測試'::pdb.jieba('chinese_convert=t2s','remove_short=2','remove_long=10')::text[];

-- ERROR TESTS
SELECT '繁體中文測試'::pdb.jieba('chinese_convert=t2st')::text[];

-- Cleanup
DROP INDEX test_chinese_bm25;
DROP TABLE test_chinese_convert;
