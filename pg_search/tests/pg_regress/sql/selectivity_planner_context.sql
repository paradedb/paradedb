-- Join and aggregate estimates must use PostgreSQL statistics for non-text filters.
\i common/common_setup.sql
SET max_parallel_workers_per_gather = 0;

CREATE TABLE estimate_products AS
SELECT i AS id, (i % 20) + 1 AS customer_id, 'widget'::text AS description,
       'category ' || (i % 4) AS category,
       CASE WHEN i % 10 = 0 THEN 0 ELSE 100 END AS price,
       1::numeric(8, 2) AS amount
FROM generate_series(1, 200) i;
CREATE TABLE estimate_customers AS SELECT i AS id FROM generate_series(1, 20) i;
CREATE INDEX estimate_products_idx ON estimate_products USING paradedb
(id, customer_id, description, (category::pdb.unicode_words('columnar=true')), price, amount);
CREATE INDEX estimate_customers_idx ON estimate_customers USING paradedb (id);
ANALYZE estimate_products;
ANALYZE estimate_customers;

-- Parsed text uses term statistics; an unsupported range falls back only for that leaf.
DO $$
DECLARE
    test record;
    plan json;
BEGIN
    FOR test IN SELECT * FROM (VALUES
        ('description @@@ ''widget''', 200),
        ('id @@@ pdb.parse(''description:widget'')', 200),
        ('description @@@ ''(widget OR missing)^2''', 200),
        ('id @@@ pdb.parse(''description:widget OR price:[100 TO 200]'')', 200),
        ('id @@@ pdb.parse(''description:widget AND price:[100 TO 200]'')', 1),
        ('id @@@ pdb.parse(''price:[100 TO 200]'')', 1)
    ) AS cases(predicate, expected_rows)
    LOOP
        EXECUTE 'EXPLAIN (FORMAT JSON) SELECT id FROM estimate_products WHERE ' || test.predicate
            INTO plan;
        IF (plan->0->'Plan'->>'Plan Rows')::int IS DISTINCT FROM test.expected_rows THEN
            RAISE EXCEPTION 'Wrong estimate for %: %', test.predicate, plan;
        END IF;
    END LOOP;
END
$$;

-- Enough rows match to group on ordinals before decoding the category strings.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(amount) FROM estimate_products
WHERE id @@@ pdb.all() AND price >= 100
GROUP BY category ORDER BY category;
SELECT category, SUM(amount) FROM estimate_products
WHERE id @@@ pdb.all() AND price >= 100
GROUP BY category ORDER BY category;

-- The customers are smaller than the matching products, so build the hash table on them.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, c.id FROM estimate_products p JOIN estimate_customers c ON p.customer_id = c.id
WHERE p.description ||| 'widget' AND p.price >= 100
ORDER BY p.id LIMIT 3;
SELECT p.id, c.id FROM estimate_products p JOIN estimate_customers c ON p.customer_id = c.id
WHERE p.description ||| 'widget' AND p.price >= 100
ORDER BY p.id LIMIT 3;

-- Parsing the same common term must also build the hash table on the smaller customers.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, c.id FROM estimate_products p JOIN estimate_customers c ON p.customer_id = c.id
WHERE p.description @@@ 'widget' AND p.price >= 100
ORDER BY p.id LIMIT 3;

DROP TABLE estimate_products, estimate_customers;

-- Both reconstructed filters and heap filters must use extended statistics.
CREATE TABLE estimate_correlated AS
SELECT i AS id, i % 3 AS a, i % 3 AS b, i % 3 AS heap_value,
       'kind ' || (i % 4) AS kind, 1::numeric(8, 2) AS amount,
       CASE WHEN i % 2 = 0 THEN 'widget' ELSE 'gadget' END AS description
FROM generate_series(1, 120) i;
CREATE INDEX estimate_correlated_idx ON estimate_correlated USING paradedb
(id, a, b, (kind::pdb.literal), description, amount);
CREATE STATISTICS estimate_dependencies (dependencies) ON a, b FROM estimate_correlated;
ANALYZE estimate_correlated;

DO $$
DECLARE
    plan json;
BEGIN
    EXECUTE $query$EXPLAIN (FORMAT JSON) SELECT id FROM estimate_correlated
        WHERE id @@@ paradedb.boolean(must => ARRAY[
            paradedb.term('a', 1), paradedb.term('b', 1)])$query$ INTO plan;
    IF (plan->0->'Plan'->>'Plan Rows')::int IS DISTINCT FROM 40 THEN
        RAISE EXCEPTION 'Functional dependencies were ignored: %', plan;
    END IF;
END
$$;

DROP STATISTICS estimate_dependencies;
CREATE STATISTICS estimate_mcv (mcv) ON a, b, heap_value FROM estimate_correlated;
ANALYZE estimate_correlated;

DO $$
DECLARE
    test record;
    plan json;
BEGIN
    FOR test IN SELECT * FROM (VALUES
        ('id @@@ paradedb.boolean(must => ARRAY[paradedb.term(''a'', 1), paradedb.term(''b'', 1)])', 40),
        ('id @@@ paradedb.boolean(should => ARRAY[paradedb.term(''a'', 1), paradedb.term(''b'', 1)])', 40),
        ('id @@@ paradedb.boolean(must => ARRAY[paradedb.term(''a'', 1), paradedb.term(''b'', 1), paradedb.term(''description'', ''widget'')])', 20),
        ('id @@@ paradedb.boolean(should => ARRAY[paradedb.term(''a'', 1), paradedb.term(''b'', 1), paradedb.term(''description'', ''widget'')])', 80),
        ('id @@@ pdb.all() AND a = 1 AND heap_value = 1', 40),
        ('id @@@ paradedb.term(''description'', ''widget'') AND a = 1 AND heap_value = 1', 20)
    ) AS cases(predicate, expected_rows)
    LOOP
        EXECUTE 'EXPLAIN (FORMAT JSON) SELECT id FROM estimate_correlated WHERE ' || test.predicate
            INTO plan;
        IF (plan->0->'Plan'->>'Plan Rows')::int IS DISTINCT FROM test.expected_rows THEN
            RAISE EXCEPTION 'Wrong extended-statistics estimate for %: %', test.predicate, plan;
        END IF;
    END LOOP;
END
$$;

-- Aggregate planning estimates the combined indexed and heap-filter query.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT kind, SUM(amount) FROM estimate_correlated
WHERE id @@@ paradedb.term('a', 1) AND heap_value = 1
GROUP BY kind ORDER BY kind;
SELECT kind, SUM(amount) FROM estimate_correlated
WHERE id @@@ paradedb.term('a', 1) AND heap_value = 1
GROUP BY kind ORDER BY kind;

DROP TABLE estimate_correlated;
