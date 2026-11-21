CREATE EXTENSION IF NOT EXISTS vector; 
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Tests that nested score modifiers correctly classify unclassified string queries.

CREATE TABLE score_docs (id int PRIMARY KEY, body text); 
INSERT INTO score_docs VALUES (1, 'hello'), (2, 'other'); 
CREATE INDEX score_docs_search ON score_docs USING paradedb (id, body); 

-- The planner requires statistics to choose the index
ANALYZE score_docs;  

-- Single constant-score modifier
SELECT id, pdb.score(id) FROM score_docs WHERE body === ('hello'::pdb.const(3)::pdb.query) ORDER BY id;

-- Boost the text query, then set a constant score
SELECT id, pdb.score(id) FROM score_docs WHERE body === ('hello'::pdb.boost(2)::pdb.query::pdb.const(3)::pdb.query) ORDER BY id;

-- Already classified query, with the nested score modifiers
SELECT id, pdb.score(id) FROM score_docs WHERE body === (pdb.term('hello')::pdb.boost(2)::pdb.query::pdb.const(3)::pdb.query) ORDER BY id;

DROP TABLE score_docs;
