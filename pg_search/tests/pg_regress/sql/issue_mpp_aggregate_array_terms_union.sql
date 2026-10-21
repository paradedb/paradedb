-- pdb.agg `terms` over an array column, on a join, under MPP.
--
-- An array facet needs the array unnested while a scalar facet in the same
-- query must not see the unnested rows, so the aggregate plan is a union of one
-- pipeline per unnest level, each over its own clone of the join. Under MPP the
-- union's children run on disjoint tasks of the top stage, each with a
-- child-local task numbering that restarts at zero. Two invariants make it run:
--
-- 1. Output partition routing is demand-driven: each `ExecuteTaskFrame` names
--    the requesting `sender_proc` and partition range, so producer fragments
--    stream directly back to whichever worker runs that union child task,
--    independent of the child's local task numbering.
-- 2. Each clone of the join claims from its own segment pool. Shared pools made
--    every clone see part of the table: silently wrong counts.
--
-- Every MPP case is compared against the serial run of the same query.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS mpp_arr_posts CASCADE;
DROP TABLE IF EXISTS mpp_arr_authors CASCADE;

CREATE TABLE mpp_arr_authors (
    id INT PRIMARY KEY,
    name TEXT,
    country TEXT,
    tags TEXT[]
);

CREATE TABLE mpp_arr_posts (
    id INT PRIMARY KEY,
    author_id INT,
    title TEXT,
    category TEXT,
    keywords TEXT[],
    views INT
);

CREATE INDEX mpp_arr_authors_idx ON mpp_arr_authors
    USING paradedb (id, name, (country::pdb.literal), (tags::pdb.literal));

CREATE INDEX mpp_arr_posts_idx ON mpp_arr_posts
    USING paradedb (id, author_id, title, (category::pdb.literal), (keywords::pdb.literal), views);

-- Two insert batches with `global_mutable_segment_rows = 0` so each index has 2
-- segments and passes the MPP launch gate (>= 2 producer tasks).
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_arr_authors VALUES
    (1, 'Alice', 'US', ARRAY['ml', 'systems']),
    (2, 'Bob',   'UK', ARRAY['web', 'db']),
    (3, 'Carol', 'US', ARRAY['ml', 'search']);

INSERT INTO mpp_arr_authors VALUES
    (4, 'Dave',  'CA', ARRAY['systems', 'search']),
    (5, 'Erin',  'DE', ARRAY[]::TEXT[]),
    (6, 'Faye',  'FR', NULL);

INSERT INTO mpp_arr_posts VALUES
    (10, 1, 'rust search engine',  'tech',    ARRAY['perf', 'index'], 100),
    (20, 2, 'rust web framework',  'web',     ARRAY['http'],          50),
    (30, 1, 'rust query compiler', 'tech',    ARRAY['perf'],          80),
    (40, 3, 'rust vector index',   'science', ARRAY['index', 'ann'],  70);

INSERT INTO mpp_arr_posts VALUES
    (50, 4, 'rust storage engine', 'tech',    ARRAY['disk'],          60),
    (60, 3, 'rust hybrid search',  'science', ARRAY['index'],         90),
    (70, 5, 'rust docs site',      'docs',    ARRAY[]::TEXT[],        10),
    (80, 6, 'rust blog post',      'docs',    NULL,                   20);

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_arr_authors;
ANALYZE mpp_arr_posts;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

-- ============================================================================
-- Serial reference values. Each case's MPP run below compares against these.
-- ============================================================================
SET max_parallel_workers_per_gather = 0;

-- C1: scalar + array facet
EXPLAIN (COSTS OFF)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c1_

-- C3: three facets, two of them arrays (three union children)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text AS author_tags,
    pdb.agg('{"terms": {"field": "keywords", "size": 10}}')::text AS keywords
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c3_

-- C6: array facet with a metric sub-aggregation
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"views": {"sum": {"field": "views"}}}}')::text AS tag_views
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c6_

-- C7: scalar outer, array inner
SELECT
    pdb.agg('{"terms": {"field": "country", "size": 10}, "aggs": {"by_tag": {"terms": {"field": "tags", "size": 10}}}}')::text AS country_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c7_

-- C8: array outer, scalar inner
SELECT
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"by_category": {"terms": {"field": "category", "size": 10}}}}')::text AS tag_categories
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c8_

-- C9: array outer, array inner (two unnest levels)
SELECT
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"by_keyword": {"terms": {"field": "keywords", "size": 10}}}}')::text AS tag_keywords
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c9_

-- C10: SQL GROUP BY alongside scalar + array facets
SELECT string_agg(format('%s|%s|%s|%s', country, n, categories, author_tags), ';' ORDER BY country) AS rows
FROM (
    SELECT
        a.country,
        count(*) AS n,
        pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
        pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
    FROM mpp_arr_posts p
    JOIN mpp_arr_authors a ON p.author_id = a.id
    WHERE p.title ||| 'rust'
      AND a.id @@@ pdb.all()
    GROUP BY a.country
) g \gset c10_

-- C11: size and min_doc_count trimming happens after the union merge
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 2}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 2, "min_doc_count": 3}}')::text AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all() \gset c11_

-- C12: a heap-filter predicate on a pdb.literal column compiles to a runtime
-- expression each union branch's scan solves in its own ExprContext.
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all()
  AND a.country = 'US' \gset c12_

-- C13: the same shape with a parameter, forced to a generic plan so the
-- predicate stays a Param until execution.
SET plan_cache_mode TO force_generic_plan;
PREPARE mpp_arr_c13(text) AS
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| $1
  AND a.id @@@ pdb.all();
EXECUTE mpp_arr_c13('rust') \gset c13_

-- ============================================================================
-- MPP runs
-- ============================================================================
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
-- A misroute used to surface as a detach error or a hang; keep a hang from
-- stalling the suite.
SET statement_timeout TO '20s';

-- C0: scalar-only facets take the single grouping-sets aggregate, no union.
EXPLAIN (COSTS OFF)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "country", "size": 10}}') AS countries
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "country", "size": 10}}') AS countries
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C1: scalar + array facet. The top stage is a DistributedUnionExec whose
-- second child runs as outer task 2 while calling itself task 0.
EXPLAIN (COSTS OFF)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c1_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c1_author_tags' AS author_tags_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C2: the same facets listed the other way round, so the scalar child is the
-- one whose child-local numbering differs from the outer one.
SELECT
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c1_author_tags' AS author_tags_match,
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c1_categories' AS categories_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C3: three union children.
EXPLAIN (COSTS OFF)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags,
    pdb.agg('{"terms": {"field": "keywords", "size": 10}}') AS keywords
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c3_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c3_author_tags' AS author_tags_match,
    pdb.agg('{"terms": {"field": "keywords", "size": 10}}')::text = :'c3_keywords' AS keywords_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C4: three union children on two tasks: a child with no task share of its
-- own is packed into another child's slot.
SET max_parallel_workers_per_gather TO 2;

EXPLAIN (COSTS OFF)
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags,
    pdb.agg('{"terms": {"field": "keywords", "size": 10}}') AS keywords
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c3_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c3_author_tags' AS author_tags_match,
    pdb.agg('{"terms": {"field": "keywords", "size": 10}}')::text = :'c3_keywords' AS keywords_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

SET max_parallel_workers_per_gather TO 3;

-- C5: leader does not participate.
SET parallel_leader_participation TO off;

SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c1_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c1_author_tags' AS author_tags_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

RESET parallel_leader_participation;

-- C6: array facet with a metric sub-aggregation.
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c6_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"views": {"sum": {"field": "views"}}}}')::text = :'c6_tag_views' AS tag_views_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C7: scalar outer, array inner.
SELECT
    pdb.agg('{"terms": {"field": "country", "size": 10}, "aggs": {"by_tag": {"terms": {"field": "tags", "size": 10}}}}')::text = :'c7_country_tags' AS country_tags_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C8: array outer, scalar inner.
SELECT
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"by_category": {"terms": {"field": "category", "size": 10}}}}')::text = :'c8_tag_categories' AS tag_categories_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C9: array outer, array inner.
SELECT
    pdb.agg('{"terms": {"field": "tags", "size": 10}, "aggs": {"by_keyword": {"terms": {"field": "keywords", "size": 10}}}}')::text = :'c9_tag_keywords' AS tag_keywords_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C10: SQL GROUP BY alongside the facets.
SELECT string_agg(format('%s|%s|%s|%s', country, n, categories, author_tags), ';' ORDER BY country) = :'c10_rows' AS rows_match
FROM (
    SELECT
        a.country,
        count(*) AS n,
        pdb.agg('{"terms": {"field": "category", "size": 10}}') AS categories,
        pdb.agg('{"terms": {"field": "tags", "size": 10}}') AS author_tags
    FROM mpp_arr_posts p
    JOIN mpp_arr_authors a ON p.author_id = a.id
    WHERE p.title ||| 'rust'
      AND a.id @@@ pdb.all()
    GROUP BY a.country
) g;

-- C11: size and min_doc_count.
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 2}}')::text = :'c11_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 2, "min_doc_count": 3}}')::text = :'c11_author_tags' AS author_tags_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all();

-- C12: heap-filter predicate; each branch's scan solves it in its own context.
SELECT
    pdb.agg('{"terms": {"field": "category", "size": 10}}')::text = :'c12_categories' AS categories_match,
    pdb.agg('{"terms": {"field": "tags", "size": 10}}')::text = :'c12_author_tags' AS author_tags_match
FROM mpp_arr_posts p
JOIN mpp_arr_authors a ON p.author_id = a.id
WHERE p.title ||| 'rust'
  AND a.id @@@ pdb.all()
  AND a.country = 'US';

-- C13: parameterized generic plan.
EXECUTE mpp_arr_c13('rust') \gset m13_
SELECT :'m13_categories' = :'c13_categories' AS categories_match,
       :'m13_author_tags' = :'c13_author_tags' AS author_tags_match;

-- The match rows above would also be true if a case had silently run serially.
-- EXPLAIN (ANALYZE, VERBOSE) prints an "MPP Launch" line only when workers were
-- launched; record it for the shapes that matter: two children on three tasks, three
-- children on two tasks, the leader sitting out, and runtime expressions.
CREATE TEMP TABLE mpp_arr_outcome (line text);
DO $$
DECLARE
    c record;
    r record;
    plan text;
BEGIN
    FOR c IN
        SELECT * FROM (VALUES
            ('C1 two children, three tasks', '3', 'on', $q$
                SELECT pdb.agg('{"terms": {"field": "category", "size": 10}}'),
                       pdb.agg('{"terms": {"field": "tags", "size": 10}}')
                FROM mpp_arr_posts p JOIN mpp_arr_authors a ON p.author_id = a.id
                WHERE p.title ||| 'rust' AND a.id @@@ pdb.all()$q$),
            ('C4 three children, two tasks', '2', 'on', $q$
                SELECT pdb.agg('{"terms": {"field": "category", "size": 10}}'),
                       pdb.agg('{"terms": {"field": "tags", "size": 10}}'),
                       pdb.agg('{"terms": {"field": "keywords", "size": 10}}')
                FROM mpp_arr_posts p JOIN mpp_arr_authors a ON p.author_id = a.id
                WHERE p.title ||| 'rust' AND a.id @@@ pdb.all()$q$),
            ('C5 leader not participating', '3', 'off', $q$
                SELECT pdb.agg('{"terms": {"field": "category", "size": 10}}'),
                       pdb.agg('{"terms": {"field": "tags", "size": 10}}')
                FROM mpp_arr_posts p JOIN mpp_arr_authors a ON p.author_id = a.id
                WHERE p.title ||| 'rust' AND a.id @@@ pdb.all()$q$),
            ('C12 heap-filter predicate', '3', 'on', $q$
                SELECT pdb.agg('{"terms": {"field": "category", "size": 10}}'),
                       pdb.agg('{"terms": {"field": "tags", "size": 10}}')
                FROM mpp_arr_posts p JOIN mpp_arr_authors a ON p.author_id = a.id
                WHERE p.title ||| 'rust' AND a.id @@@ pdb.all() AND a.country = 'US'$q$),
            ('C13 parameterized generic plan', '3', 'on', $q$EXECUTE mpp_arr_c13('rust')$q$)
        ) AS cases(label, workers, leader, sql)
    LOOP
        PERFORM set_config('max_parallel_workers_per_gather', c.workers, true);
        PERFORM set_config('parallel_leader_participation', c.leader, true);
        plan := '';
        FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF) ' || c.sql LOOP
            plan := plan || r."QUERY PLAN" || E'\n';
        END LOOP;
        IF plan LIKE '%MPP Launch:%' THEN
            INSERT INTO mpp_arr_outcome VALUES (c.label || ': MPP launched');
        ELSE
            INSERT INTO mpp_arr_outcome VALUES (c.label || ': UNEXPECTED serial run');
        END IF;
    END LOOP;
END$$;
SELECT line FROM mpp_arr_outcome ORDER BY line;
DROP TABLE mpp_arr_outcome;
RESET parallel_leader_participation;
SET max_parallel_workers_per_gather TO 3;

RESET statement_timeout;
DEALLOCATE mpp_arr_c13;
RESET plan_cache_mode;

-- Teardown
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;
RESET min_parallel_index_scan_size;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;

DROP TABLE mpp_arr_posts CASCADE;
DROP TABLE mpp_arr_authors CASCADE;
