CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE test (id int, content text, extra jsonb);
CREATE INDEX test_bm25 ON test USING bm25 (id, content, extra) WITH (key_field='id');

INSERT INTO test (id, content, extra) VALUES
(1, 'Beijing CBD area', '{"type": "business", "district": "Chaoyang"}'::jsonb),
(2, 'Beijing Palace Museum', '{"type": "landmark", "district": "Dongcheng"}'::jsonb),
(3, 'Shanghai Bund', '{"type": "tourism", "district": "Huangpu"}'::jsonb),
(4, 'Universal Studios Beijing', '{"type": "entertainment", "district": "Tongzhou"}'::jsonb);

EXPLAIN (COSTS OFF)
SELECT pdb.score(test.id), test.content, test.extra
FROM test
WHERE (test.content &&& 'Beijing') AND jsonb_path_exists(test.extra, '$.type')
ORDER BY pdb.score(test.id) DESC
LIMIT 10;

SELECT pdb.score(test.id), test.content, test.extra
FROM test
WHERE (test.content &&& 'Beijing') AND jsonb_path_exists(test.extra, '$.type')
ORDER BY pdb.score(test.id) DESC
LIMIT 10;
