BEGIN;
CREATE SCHEMA sequential_scan_subquery;
SET search_path = sequential_scan_subquery, public;

CREATE TABLE products (id bigint PRIMARY KEY, name text, rating int, color text);
CREATE TABLE orders (id bigint PRIMARY KEY, color text, rating int, name text);
CREATE INDEX products_search ON products USING paradedb
    (id, (name::pdb.literal), (color::pdb.literal));
CREATE INDEX orders_search ON orders USING paradedb
    (id, (name::pdb.literal), (color::pdb.literal));

-- The same CTID names different rows in the two tables.
INSERT INTO products VALUES (1, 'bob', 4, 'blue'), (2, 'alice', 4, 'red');
INSERT INTO orders (id, name, rating, color) VALUES (1, 'alice', 4, 'red'), (2, 'bob', 4, 'blue');
ANALYZE products;
ANALYZE orders;
SET paradedb.enable_custom_scan = off;
SET paradedb.enable_custom_scan_without_operator = off;
SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.enable_join_custom_scan = off;
SET paradedb.enable_filter_pushdown = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;

SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name = 'bob'))
      AND products.rating = 4 OR products.color = 'blue';
SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name === 'bob'))
      AND products.rating = 4 OR products.color === 'blue';

SELECT array_agg(products.id ORDER BY products.id) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND orders.name === 'bob')
      OR products.id = -1;

SELECT array_agg(products.id ORDER BY products.id) FROM products
WHERE EXISTS (SELECT 1 FROM (SELECT name, color FROM orders) AS projected
              WHERE projected.name = products.name AND projected.name === 'bob')
      OR products.id = -1;

SELECT array_agg(products.id ORDER BY products.id) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name
                    AND orders.name @@@ paradedb.term('color', 'red'))
      OR products.id = -1;

DO $$
DECLARE
    plan json;
BEGIN
    EXECUTE $query$EXPLAIN (FORMAT JSON, COSTS OFF)
        SELECT count(*) FROM products
        WHERE EXISTS (SELECT 1 FROM orders
                      WHERE orders.name = products.name AND NOT (orders.name === 'bob'))
              AND products.rating = 4 OR products.color === 'blue'$query$ INTO plan;
    ASSERT plan #>> '{0,Plan,Plans,0,Node Type}' = 'Seq Scan';
    ASSERT plan #>> '{0,Plan,Plans,0,Filter}' LIKE '%hashed SubPlan%',
        'the regression must exercise hashed EXISTS conversion';
END;
$$;

-- A generic plan keeps the RHS as an expression rather than a query constant.
SET plan_cache_mode = force_generic_plan;
PREPARE lookup(text) AS
SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name === $1))
      AND products.rating = 4 OR products.color === 'blue';
EXECUTE lookup('bob');
EXECUTE lookup('alice');
DEALLOCATE lookup;

-- Rebinding must not discard a user-supplied index expression with side effects.
CREATE SEQUENCE index_lookup_calls;
SELECT count(*) FROM products
WHERE id @@@ paradedb.with_index(
    CASE WHEN nextval('index_lookup_calls') > 0
         THEN 'products_search'::regclass ELSE 'orders_search'::regclass END,
    paradedb.term('color', 'blue')
);
SELECT is_called FROM index_lookup_calls;

CREATE VIEW saved_exists AS
SELECT products.id FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND orders.name === 'bob')
      OR products.id = -1;
SELECT array_agg(id ORDER BY id) FROM saved_exists;

DO $$
DECLARE
    op text;
    rhs_type text;
    input text;
    matches bigint[];
BEGIN
    FOREACH op IN ARRAY ARRAY['===', '&&&', '|||', '###', '@@@'] LOOP
        FOREACH rhs_type IN ARRAY ARRAY['text', 'text[]'] LOOP
            CONTINUE WHEN op = '@@@' AND rhs_type = 'text[]';
            input := CASE WHEN rhs_type = 'text' THEN 'bob' ELSE '{bob}' END;
            EXECUTE format($query$
                SELECT array_agg(p.id ORDER BY p.id) FROM products p
                WHERE EXISTS (SELECT 1 FROM orders c
                              WHERE c.name = p.name AND c.name %s %L::%s)
                      OR p.id = -1$query$, op, input, rhs_type) INTO matches;
            ASSERT matches IS NOT DISTINCT FROM ARRAY[1::bigint], op || ' constant ' || rhs_type;
            EXECUTE format($query$
                PREPARE inferred_lookup(%s) AS
                SELECT array_agg(p.id ORDER BY p.id) FROM products p
                WHERE EXISTS (SELECT 1 FROM orders c
                              WHERE c.name = p.name AND c.name %s $1)
                      OR p.id = -1$query$, rhs_type, op);
            EXECUTE format('EXECUTE inferred_lookup(%L)', input) INTO matches;
            ASSERT matches IS NOT DISTINCT FROM ARRAY[1::bigint], op || ' generic ' || rhs_type;
            DEALLOCATE inferred_lookup;
        END LOOP;
    END LOOP;
END;
$$;

PREPARE inferred_lookup(pdb.query) AS
SELECT array_agg(products.id ORDER BY products.id) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND orders.name === $1)
      OR products.id = -1;
EXECUTE inferred_lookup('bob'::pdb.query);
DEALLOCATE inferred_lookup;

SELECT array_agg(p.id ORDER BY p.id) FROM products p
WHERE EXISTS (SELECT 1 FROM orders c
              WHERE c.name = p.name AND c.name === 'bob'
                    AND EXISTS (SELECT 1 FROM products nested
                                WHERE nested.name = c.name AND nested.name === 'bob'))
      OR p.id = -1;

INSERT INTO products (id, name) VALUES (3, NULL);
SELECT id, name @@@ pdb.exists() AS present, (SELECT 1) AS subquery
FROM products ORDER BY id;

-- Both helper variants must rebind fields before evaluating rows outside the index.
DO $$
DECLARE
    nullable_anchor boolean;
    fields text;
    plan json;
    matches bigint[];
    query_sql text := 'SELECT array_agg(p.id ORDER BY p.id) FROM products p
                      WHERE EXISTS (SELECT 1 FROM orders c
                                    WHERE c.name = p.name AND c.name === %s)
                            OR p.id = -1';
BEGIN
    FOREACH nullable_anchor IN ARRAY ARRAY[false, true] LOOP
        DROP INDEX products_search;
        DROP INDEX orders_search;
        fields := CASE WHEN nullable_anchor THEN 'color, id, (name::pdb.literal)'
                       ELSE 'id, (name::pdb.literal), (color::pdb.literal)' END;
        EXECUTE format('CREATE INDEX products_search ON products USING paradedb (%s) WHERE id < 0', fields);
        EXECUTE format('CREATE INDEX orders_search ON orders USING paradedb (%s) WHERE id < 0', fields);
        EXECUTE format(query_sql, quote_literal('bob')) INTO matches;
        ASSERT matches IS NOT DISTINCT FROM ARRAY[1::bigint], 'constant inline field binding';
        EXECUTE 'PREPARE original_lhs_lookup(text) AS ' || format(query_sql, '$1');
        EXECUTE 'EXECUTE original_lhs_lookup(''bob'')' INTO matches;
        ASSERT matches IS NOT DISTINCT FROM ARRAY[1::bigint], 'generic inline field binding';
        EXECUTE 'EXECUTE original_lhs_lookup(''alice'')' INTO matches;
        ASSERT matches IS NOT DISTINCT FROM ARRAY[2::bigint], 'generic inline field binding';
        EXECUTE 'EXPLAIN (FORMAT JSON, COSTS OFF) EXECUTE original_lhs_lookup(''bob'')' INTO plan;
        ASSERT plan::text LIKE '%hashed SubPlan%', 'must exercise hashed EXISTS conversion';
        ASSERT plan::text LIKE CASE WHEN nullable_anchor
                                   THEN '%search_with_query_input_ctid_or_row(%'
                                   ELSE '%search_with_query_input_ctid_or_row_strict(%' END,
               'must exercise the expected inline helper';
        DEALLOCATE original_lhs_lookup;
    END LOOP;
END;
$$;

DROP INDEX products_search;
DROP INDEX orders_search;
CREATE INDEX products_search ON products USING paradedb
    (id, (lower(color)::pdb.literal('alias=product_color')));
CREATE INDEX orders_search ON orders USING paradedb
    (id, (lower(name)::pdb.literal('alias=order_name')));
SELECT array_agg(products.id ORDER BY products.id) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND lower(orders.name) === 'bob')
      OR products.id = -1;
ROLLBACK;
