-- Test ltree type support in BM25 indexes
-- ltree is a PostgreSQL extension type for hierarchical tree-like structures

CREATE EXTENSION IF NOT EXISTS ltree;
CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Basic ltree support test
DROP TABLE IF EXISTS tbl_ltree;
CREATE TABLE tbl_ltree (
    id SERIAL,
    category ltree
);
CREATE INDEX idx_ltree ON tbl_ltree USING bm25 (id, category) WITH (key_field = 'id');

-- Insert test data with various ltree paths
INSERT INTO tbl_ltree (category) VALUES 
    ('Top.Science.Astronomy'),
    ('Top.Science.Biology'),
    ('Top.Science.Biology.Botany'),
    ('Top.Collections.Pictures'),
    ('Top.Collections.Pictures.Astronomy'),
    ('Top.Hobbies.Photography'),
    (NULL);

-- Test equality query via BM25 index
SELECT id, category FROM tbl_ltree WHERE category @@@ 'Top.Science.Astronomy' ORDER BY id;

-- Test count aggregation with ltree filter
SELECT count(*) FROM tbl_ltree WHERE category @@@ 'Top.Science.Biology';

-- Explain to verify index usage
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM tbl_ltree WHERE category @@@ 'Top.Science.Biology';

-- Test sorting by ltree column (lexicographic order)
SELECT id, category FROM tbl_ltree WHERE id @@@ pdb.all() ORDER BY category ASC NULLS LAST;

-- Test ltree as key field
DROP TABLE IF EXISTS tbl_ltree_key;
CREATE TABLE tbl_ltree_key (
    path ltree,
    name TEXT
);
CREATE INDEX idx_ltree_key ON tbl_ltree_key USING bm25 (path, name) WITH (key_field = 'path');

INSERT INTO tbl_ltree_key (path, name) VALUES 
    ('Root.Branch1', 'First Branch'),
    ('Root.Branch2', 'Second Branch');

SELECT path, name FROM tbl_ltree_key WHERE name @@@ 'Branch' ORDER BY path;

RESET paradedb.enable_aggregate_custom_scan;
