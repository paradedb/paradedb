CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS snippet_test;
CREATE TABLE snippet_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO snippet_test (content) VALUES
('This is a test test of the snippet function with multiple test words'),
('Another test of the snippet snippet function with repeated snippet words'),
('Yet another test test test of the function function function'),
('test Lorem ipsum dolor sit amet...test');

CREATE INDEX ON snippet_test USING bm25 (
    id,
    content
) WITH (
    key_field = 'id'
);

SELECT pdb.snippet(content), pdb.snippet_positions(content) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "limit" => 1), pdb.snippet_positions(content, "limit" => 1) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "limit" => 1, "offset" => 1), pdb.snippet_positions(content, "limit" => 1, "offset" => 1) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "limit" => 5, "offset" => 2), pdb.snippet_positions(content, "limit" => 5, "offset" => 2) FROM snippet_test WHERE content @@@ 'test';

-- Edge cases
SELECT pdb.snippet(content, "limit" => 0), pdb.snippet_positions(content, "limit" => 0) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "limit" => -1), pdb.snippet_positions(content, "limit" => -1) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "offset" => 1000), pdb.snippet_positions(content, "offset" => 1000) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, "limit" => null), pdb.snippet_positions(content, "limit" => null) FROM snippet_test WHERE content @@@ 'test';

-- With max num chars
SELECT pdb.snippet(content, max_num_chars => 20, "offset" => 2) FROM snippet_test WHERE content @@@ 'test';
SELECT pdb.snippet(content, max_num_chars => 0, "offset" => 2) FROM snippet_test WHERE content @@@ 'test';

DROP TABLE snippet_test;
