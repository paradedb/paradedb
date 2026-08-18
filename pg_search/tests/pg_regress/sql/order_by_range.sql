-- Tests `ORDER BY <range column>` pushdown into the BM25 index (issue #2688).
--
-- Range columns are indexed as a tantivy JSON object with `lower`/`upper`/`empty` and the four
-- bound flags as fast field sub-columns. `SortByRange` reads those and assembles a composite
-- sort key ordered exactly like Postgres' `range_cmp`: empty first, unbounded lower first,
-- unbounded upper last, inclusive-before-exclusive on an equal lower bound, and
-- exclusive-before-inclusive on an equal upper bound.
--
-- The correctness oracle throughout is native Postgres: every pushed-down result is compared
-- against the same query with `paradedb.enable_custom_scan = off`.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

CREATE TABLE range_items (
    id SERIAL PRIMARY KEY,
    title TEXT,
    i4  INT4RANGE,
    i8  INT8RANGE,
    nr  NUMRANGE,
    dr  DATERANGE,
    tr  TSRANGE,
    tzr TSTZRANGE
);

-- Every interesting shape: empty, fully unbounded, half unbounded, inclusive/exclusive
-- variations on the same bound values, a duplicate row, and SQL NULL.
INSERT INTO range_items (title, i4, i8, nr, dr, tr, tzr) VALUES
 ('doc', 'empty', 'empty', 'empty', 'empty', 'empty', 'empty'),
 ('doc', '(,)', '(,)', '(,)', '(,)', '(,)', '(,)'),
 ('doc', '(,10)', '(,10)', '(,10)', '(,2020-01-10)', '(,2020-01-10)', '(,2020-01-10)'),
 ('doc', '[4,10)', '[4,10)', '[4,10)', '[2020-01-04,2020-01-10)', '[2020-01-04,2020-01-10)', '[2020-01-04,2020-01-10)'),
 ('doc', '[5,10)', '[5,10)', '[5,10)', '[2020-01-05,2020-01-10)', '[2020-01-05,2020-01-10)', '[2020-01-05,2020-01-10)'),
 ('doc', '[5,10]', '[5,10]', '[5,10]', '[2020-01-05,2020-01-10]', '[2020-01-05,2020-01-10]', '[2020-01-05,2020-01-10]'),
 ('doc', '[5,11)', '[5,11)', '[5,11)', '[2020-01-05,2020-01-11)', '[2020-01-05,2020-01-11)', '[2020-01-05,2020-01-11)'),
 ('doc', '[5,)', '[5,)', '[5,)', '[2020-01-05,)', '[2020-01-05,)', '[2020-01-05,)'),
 ('doc', '(5,10)', '(5,10)', '(5,10)', '(2020-01-05,2020-01-10)', '(2020-01-05,2020-01-10)', '(2020-01-05,2020-01-10)'),
 ('doc', '(5,10]', '(5,10]', '(5,10]', '(2020-01-05,2020-01-10]', '(2020-01-05,2020-01-10]', '(2020-01-05,2020-01-10]'),
 ('doc', NULL, NULL, NULL, NULL, NULL, NULL),
 ('doc', '[5,10)', '[5,10)', '[5,10)', '[2020-01-05,2020-01-10)', '[2020-01-05,2020-01-10)', '[2020-01-05,2020-01-10)'),
 ('doc', '[-100,-50)', '[-9223372036854775808,0)', '[-0.00001,0.5)', '[1900-01-01,1901-01-01)', '[1900-01-01,1901-01-01)', '[1900-01-01,1901-01-01)'),
 ('doc', '[0,1)', '[0,1)', '[0.0000000001,123456789.123456789)', '[2020-01-01,2020-01-02)', '[2020-01-01 00:00:00.000001,2020-01-02)', '[2020-01-01 00:00:00.000001,2020-01-02)');

-- Range fields are fast by default, so no explicit field configuration is needed.
CREATE INDEX range_items_idx ON range_items
USING paradedb (id, title, i4, i8, nr, dr, tr, tzr) WITH (key_field = 'id');

-- =============================================================================
-- The plan: ORDER BY range + LIMIT is a Top-N scan, with no Postgres Sort above it.
-- Before this was supported, the same query produced a `Sort` over a
-- `NormalScanExecState` scan and a "not using Top K scan" warning.
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM range_items WHERE title @@@ 'doc' ORDER BY tzr LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM range_items WHERE title @@@ 'doc' ORDER BY nr DESC LIMIT 5;

-- =============================================================================
-- The ordering itself, per range type. `id` is the tiebreaker so the duplicate
-- row does not make the output non-deterministic.
-- =============================================================================
SELECT id, i4 FROM range_items WHERE title @@@ 'doc' ORDER BY i4, id LIMIT 14;
SELECT id, i8 FROM range_items WHERE title @@@ 'doc' ORDER BY i8, id LIMIT 14;
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr, id LIMIT 14;
SELECT id, dr FROM range_items WHERE title @@@ 'doc' ORDER BY dr, id LIMIT 14;
SELECT id, tr FROM range_items WHERE title @@@ 'doc' ORDER BY tr, id LIMIT 14;
SELECT id, tzr FROM range_items WHERE title @@@ 'doc' ORDER BY tzr, id LIMIT 14;

-- Continuous types keep their inclusivity, so they exercise the bound-flag tiebreaks that
-- discrete types normalize away (`(5,10)` becomes `[6,10)` for int4range).
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr DESC, id LIMIT 14;
SELECT id, tzr FROM range_items WHERE title @@@ 'doc' ORDER BY tzr DESC, id LIMIT 14;

-- NULL placement follows the query, not the storage.
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr ASC NULLS FIRST, id LIMIT 3;
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr ASC NULLS LAST, id LIMIT 3;
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr DESC NULLS FIRST, id LIMIT 3;
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr DESC NULLS LAST, id LIMIT 3;

-- OFFSET walks past the leading keys.
SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY nr, id LIMIT 4 OFFSET 6;

-- =============================================================================
-- Differential check against native Postgres, for every range type crossed with
-- every direction/NULLS combination and a few limits. Compares the ordered
-- sequence of range values, so rows that tie on the sort key cannot register as
-- a difference. Must report zero mismatches and no declines.
-- =============================================================================
CREATE FUNCTION check_range_order(tbl text, col text, dir text, lim int, off int)
RETURNS text LANGUAGE plpgsql AS $$
DECLARE
    query text;
    pushed text[];
    native text[];
    line text;
    pushed_down bool := false;
BEGIN
    -- The EXPLAIN and the two EXECUTEs share one statement text on purpose. If
    -- they drifted, the plan check could pass on one shape while the values
    -- came from another, and a lost pushdown would compare native to native.
    query := format(
        'SELECT array_agg(v) FROM ('
        '  SELECT %I::text AS v FROM %I WHERE title @@@ ''doc'' ORDER BY %I %s LIMIT %s OFFSET %s'
        ') s',
        col, tbl, col, dir, lim, off);

    SET paradedb.enable_custom_scan = on;
    EXECUTE query INTO pushed;
    FOR line IN EXECUTE 'EXPLAIN (COSTS OFF) ' || query LOOP
        IF line LIKE '%TopK%' THEN
            pushed_down := true;
        END IF;
    END LOOP;

    SET paradedb.enable_custom_scan = off;
    EXECUTE query INTO native;
    SET paradedb.enable_custom_scan = on;

    IF pushed IS DISTINCT FROM native THEN
        RETURN format('MISMATCH %s %s limit %s offset %s: pushed=%s native=%s',
                      col, dir, lim, off, pushed, native);
    END IF;
    IF NOT pushed_down THEN
        RETURN format('NOT PUSHED DOWN: %s %s', col, dir);
    END IF;
    RETURN 'ok';
END $$;

SELECT result, count(*)
FROM (
    SELECT check_range_order('range_items', col, dir, lim, off) AS result
    FROM unnest(ARRAY['i4', 'i8', 'nr', 'dr', 'tr', 'tzr']) col,
         unnest(ARRAY['ASC', 'DESC', 'ASC NULLS FIRST', 'DESC NULLS LAST']) dir,
         unnest(ARRAY[1, 3, 14, 20]) lim,
         unnest(ARRAY[0, 5]) off
) s
GROUP BY result
ORDER BY result;

-- =============================================================================
-- A range is only pushed down as the *leading* sort key: later keys are collected
-- via `SortByErasedType`, which reads a single column and cannot express the
-- composite bound comparison. Range-then-column pushes; column-then-range declines
-- to a Postgres Sort, which is correct but unaccelerated.
-- =============================================================================
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM range_items WHERE title @@@ 'doc' ORDER BY nr, id LIMIT 5;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM range_items WHERE title @@@ 'doc' ORDER BY id, nr LIMIT 5;

SELECT id, nr FROM range_items WHERE title @@@ 'doc' ORDER BY id, nr LIMIT 5;

-- `lower(anyrange)` is a different function from the `lower(text)` the planner
-- recognises, and a scalar bound orders differently from the whole range, so this
-- must not be mistaken for a sort on `nr`. It stays a Postgres Sort.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM range_items WHERE title @@@ 'doc' ORDER BY lower(nr) LIMIT 5;

SELECT id, lower(nr) FROM range_items WHERE title @@@ 'doc' ORDER BY lower(nr), id LIMIT 5;

-- =============================================================================
-- Cross-segment merge. Within a segment, a `numrange` bound compares by term
-- ordinal; ordinals are not comparable across segments, so the key has to be
-- resolved to bytes before segments are merged. Several batched inserts build
-- several segments to exercise that path.
-- =============================================================================
CREATE TABLE range_segments (id SERIAL PRIMARY KEY, title TEXT, nr NUMRANGE, tzr TSTZRANGE);
CREATE INDEX range_segments_idx ON range_segments
USING paradedb (id, title, nr, tzr) WITH (key_field = 'id');

DO $$
BEGIN
    FOR batch IN 0..5 LOOP
        INSERT INTO range_segments (title, nr, tzr)
        SELECT 'doc',
               CASE WHEN i % 31 = 0 THEN NULL
                    WHEN i % 29 = 0 THEN 'empty'::numrange
                    WHEN i % 23 = 0 THEN numrange(NULL, (i % 500)::numeric / 7, '()')
                    WHEN i % 19 = 0 THEN numrange((i % 500)::numeric / 7, NULL, '[)')
                    ELSE numrange((i % 500)::numeric / 7,
                                  (i % 500)::numeric / 7 + 3,
                                  CASE WHEN i % 2 = 0 THEN '[)' ELSE '(]' END)
               END,
               CASE WHEN i % 37 = 0 THEN NULL
                    WHEN i % 17 = 0 THEN 'empty'::tstzrange
                    ELSE tstzrange('2020-01-01'::timestamptz + ((i % 300) || ' hours')::interval,
                                   '2020-01-01'::timestamptz + ((i % 300) || ' hours')::interval + '3 days')
               END
        FROM generate_series(batch * 500 + 1, (batch + 1) * 500) i;
    END LOOP;
END $$;

SELECT count(*) > 1 AS has_multiple_segments FROM paradedb.index_info('range_segments_idx');

SELECT result, count(*)
FROM (
    SELECT check_range_order('range_segments', col, dir, lim, off) AS result
    FROM unnest(ARRAY['nr', 'tzr']) col,
         unnest(ARRAY['ASC', 'DESC', 'ASC NULLS FIRST', 'DESC NULLS LAST']) dir,
         unnest(ARRAY[1, 25, 500]) lim,
         unnest(ARRAY[0, 13, 2900]) off
) s
GROUP BY result
ORDER BY result;

-- =============================================================================
-- Issue #2688's original query. It used to fail outright with
-- `SchemaError("Fast field not available: '\"valid_period\"'")`, then later
-- silently degraded to a Postgres Sort. Now it is a Top-N scan.
-- =============================================================================
CREATE TABLE data_records (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT,
    valid_period TSTZRANGE,
    quantity_range NUMRANGE
);

INSERT INTO data_records (title, category, valid_period, quantity_range)
SELECT 'Product ' || i,
       CASE WHEN i % 4 = 0 THEN 'Electronics'
            WHEN i % 4 = 1 THEN 'Clothing'
            WHEN i % 4 = 2 THEN 'Books'
            ELSE 'Home' END,
       tstzrange('2023-01-01'::timestamptz + ((i % 365) || ' days')::interval,
                 '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval + '1 month'::interval),
       numrange((i % 10) * 10, (i % 10 + 1) * 10)
FROM generate_series(1, 100) i;

CREATE INDEX records_no_fast_idx ON data_records
USING paradedb (id, title, category, valid_period, quantity_range) WITH (key_field = 'id');

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, title, category FROM data_records
WHERE title @@@ 'product' ORDER BY valid_period LIMIT 10;

SELECT id, title, category FROM data_records
WHERE title @@@ 'product' ORDER BY valid_period, id LIMIT 10;

-- =============================================================================
-- Parallel TopK. The range key is merged across workers here, not just across
-- segments. `range_segments` is the table to use: a single segment offers no
-- useful workers, so `range_items` would stay serial whatever the cost knobs say.
-- =============================================================================
SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;

CREATE FUNCTION plan_contains(query text, needle text) RETURNS bool LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (COSTS OFF) ' || query LOOP
        IF line LIKE '%' || needle || '%' THEN
            RETURN true;
        END IF;
    END LOOP;
    RETURN false;
END;
$$;

-- Guards the two checks below. Without a Gather they would silently re-test the
-- serial path, which the rest of the file already covers. The statement is the
-- same wrapped shape `check_range_order` measures, so the plan being asserted is
-- the plan whose values get compared.
SELECT plan_contains(
    'SELECT array_agg(v) FROM ('
    '  SELECT nr::text AS v FROM range_segments WHERE title @@@ ''doc'' ORDER BY nr LIMIT 5'
    ') s',
    'Gather') AS parallel_plan,
    plan_contains(
    'SELECT array_agg(v) FROM ('
    '  SELECT nr::text AS v FROM range_segments WHERE title @@@ ''doc'' ORDER BY nr LIMIT 5'
    ') s',
    'TopK') AS topk_plan;

SELECT check_range_order('range_segments', 'nr', 'ASC', 5, 0);
SELECT check_range_order('range_segments', 'tzr', 'DESC NULLS LAST', 4, 0);

RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET min_parallel_table_scan_size;

DROP FUNCTION plan_contains(text, text);
DROP FUNCTION check_range_order(text, text, text, int, int);
DROP TABLE data_records CASCADE;
DROP TABLE range_segments CASCADE;
DROP TABLE range_items CASCADE;
RESET max_parallel_workers_per_gather;
