\i common/common_setup.sql

CREATE TABLE fieldnorms_test (
    id INT,
    content pdb.simple('fieldnorms=false')
);

INSERT INTO fieldnorms_test VALUES
(1, 'this is a test'::pdb.simple),
(2, ('this is a test ' || repeat('word ', 500))::pdb.simple);


CREATE INDEX fieldnorms_test_idx
ON fieldnorms_test
USING bm25(content)
WITH (key_field = 'id');

SELECT
    id,
    paradedb.score(id)
FROM fieldnorms_test
WHERE content @@@ 'test'
ORDER BY id;

WITH scores AS (
    SELECT paradedb.score(id) as s
    FROM fieldnorms_test
    WHERE content @@@ 'test'
)
SELECT (MAX(s) - MIN(s)) < 0.00001 as scores_are_identical FROM scores;

DROP TABLE fieldnorms_test;