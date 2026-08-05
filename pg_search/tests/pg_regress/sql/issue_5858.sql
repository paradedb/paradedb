-- Regression for issue #5858. PostgreSQL nondeterministic ICU collations
-- can equate distinct byte strings (e.g. 'Electronics' and 'electronics').
-- The scan must not push byte exact Tantivy term queries in that case,
-- or it would return fewer rows than PostgreSQL semantics require.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.check_aggregate_scan = false;

DROP TABLE IF EXISTS issue_5858_products CASCADE;

CREATE COLLATION issue_5858_ci (
    provider = icu,
    locale = 'und-u-ks-level2',
    deterministic = false
);

CREATE TABLE issue_5858_products (
    id bigint PRIMARY KEY,
    name_ci text COLLATE issue_5858_ci
);

INSERT INTO issue_5858_products VALUES
    (1, 'Electronics'),
    (2, 'electronics'),
    (3, 'Books');

CREATE INDEX issue_5858_products_bm25
ON issue_5858_products
USING bm25 (id, (name_ci::pdb.literal))
WITH (key_field = 'id');

-- Native PostgreSQL under the nondeterministic collation equates
-- 'Electronics' and 'electronics'.
SELECT id FROM issue_5858_products
WHERE name_ci = 'Electronics'
ORDER BY id;

-- The same query mixed with a paradedb.all() driver must return the same
-- two rows. Before the fix, the equality qual was pushed as a Tantivy
-- byte exact term and only row 1 came back.
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci = 'Electronics'
ORDER BY id;

-- The mixed query must return the identical row set to the native path.
SELECT array_agg(id ORDER BY id) AS native_ids
FROM issue_5858_products
WHERE name_ci = 'Electronics';

SELECT array_agg(id ORDER BY id) AS mixed_ids
FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci = 'Electronics';

SELECT (SELECT array_agg(id ORDER BY id)
        FROM issue_5858_products
        WHERE name_ci = 'Electronics') =
       (SELECT array_agg(id ORDER BY id)
        FROM issue_5858_products
        WHERE id @@@ paradedb.all()
          AND name_ci = 'Electronics') AS row_sets_agree;

-- '<>' with the same nondeterministic collation must also decline pushdown.
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci <> 'Electronics'
ORDER BY id;

-- Default collation (deterministic) must still push down. Separate table so
-- the column is not tagged with the ICU collation.
CREATE TABLE issue_5858_default (
    id bigint PRIMARY KEY,
    name_txt text
);
INSERT INTO issue_5858_default VALUES
    (1, 'Electronics'),
    (2, 'electronics');
CREATE INDEX issue_5858_default_bm25
ON issue_5858_default
USING bm25 (id, (name_txt::pdb.literal))
WITH (key_field = 'id');

SELECT id FROM issue_5858_default
WHERE id @@@ paradedb.all()
  AND name_txt = 'Electronics'
ORDER BY id;

DROP TABLE issue_5858_default;
DROP TABLE issue_5858_products;
DROP COLLATION issue_5858_ci;
