-- Float filters against NaN and the infinities must match PostgreSQL. JSON has no form for
-- these values, so they used to reach the term builder as null and fail with
-- "Tantivy PdbOwnedValue type not supported".
CREATE EXTENSION IF NOT EXISTS pg_search;

-- The reference table has no ParadeDB index, so it is plain PostgreSQL.
CREATE TABLE issue_7237_plain (id int PRIMARY KEY, d double precision, r real, body text);
INSERT INTO issue_7237_plain VALUES
    (1, 'NaN', 'NaN', 'x'),
    (2, 'Infinity', 'Infinity', 'x'),
    (3, '-Infinity', '-Infinity', 'x'),
    (4, 1.5, 1.5, 'x'),
    (5, -1.5, -1.5, 'x'),
    (6, 0, 0, 'x'),
    (7, NULL, NULL, 'x');

CREATE TABLE issue_7237_t (LIKE issue_7237_plain INCLUDING ALL);
CREATE INDEX issue_7237_t_idx ON issue_7237_t USING paradedb (id, d, r, body);
INSERT INTO issue_7237_t SELECT * FROM issue_7237_plain;

-- The same rows in immutable segments, one per insert
SET paradedb.global_mutable_segment_rows = 0;
CREATE TABLE issue_7237_imm (LIKE issue_7237_plain INCLUDING ALL);
CREATE INDEX issue_7237_imm_idx ON issue_7237_imm USING paradedb (id, d, r, body);
INSERT INTO issue_7237_imm SELECT * FROM issue_7237_plain WHERE id = 1;
INSERT INTO issue_7237_imm SELECT * FROM issue_7237_plain WHERE id = 2;
INSERT INTO issue_7237_imm SELECT * FROM issue_7237_plain WHERE id = 3;
INSERT INTO issue_7237_imm SELECT * FROM issue_7237_plain WHERE id > 3;
RESET paradedb.global_mutable_segment_rows;

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

-- The issue's queries
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d = 'NaN';
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d < 'Infinity' ORDER BY id;
SELECT COUNT(*) FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d = 'NaN';

-- The filter stays pushed down, and the plan shows the constant
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d = 'NaN';
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d IN ('NaN', 'Infinity');
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d >= '-Infinity';

-- Every column, constant and operator, against the reference table. Returns only mismatches.
CREATE FUNCTION issue_7237_mismatches(tbl text) RETURNS SETOF text
LANGUAGE plpgsql AS $$
DECLARE
    col text;
    val text;
    shape text;
    pred text;
    want int[];
    got int[];
    want_count bigint;
    got_count bigint;
BEGIN
    FOREACH col IN ARRAY ARRAY['d', 'r'] LOOP
        FOREACH val IN ARRAY ARRAY['NaN', 'Infinity', '-Infinity'] LOOP
            FOREACH shape IN ARRAY ARRAY[
                '%1$I = %2$L',
                '%1$I <> %2$L',
                '%1$I < %2$L',
                '%1$I <= %2$L',
                '%1$I > %2$L',
                '%1$I >= %2$L',
                '%1$I IN (%2$L, 1.5)',
                '%1$I NOT IN (%2$L, 1.5)',
                '%1$I BETWEEN %2$L AND %2$L',
                '%1$I BETWEEN 0 AND %2$L',
                '%1$I BETWEEN %2$L AND 0'
            ] LOOP
                pred := format(shape, col, val);
                EXECUTE format(
                    'SELECT ARRAY(SELECT id FROM issue_7237_plain WHERE %s ORDER BY id)', pred
                ) INTO want;
                EXECUTE format(
                    'SELECT ARRAY(SELECT id FROM %1$I WHERE %1$I @@@ pdb.all() AND %2$s ORDER BY id)',
                    tbl, pred
                ) INTO got;
                IF got IS DISTINCT FROM want THEN
                    RETURN NEXT format('rows %s: got %s, want %s', pred, got, want);
                END IF;
                EXECUTE format('SELECT COUNT(*) FROM issue_7237_plain WHERE %s', pred)
                    INTO want_count;
                EXECUTE format(
                    'SELECT COUNT(*) FROM %1$I WHERE %1$I @@@ pdb.all() AND %2$s', tbl, pred
                ) INTO got_count;
                IF got_count IS DISTINCT FROM want_count THEN
                    RETURN NEXT format('count %s: got %s, want %s', pred, got_count, want_count);
                END IF;
            END LOOP;
        END LOOP;
    END LOOP;
END $$;

SELECT * FROM issue_7237_mismatches('issue_7237_t');
SELECT * FROM issue_7237_mismatches('issue_7237_imm');

-- Stored pdb.query values preserve non-finite constants across reads and text I/O.
CREATE TABLE issue_7237_saved_queries (expected_id int, query pdb.query);
INSERT INTO issue_7237_saved_queries VALUES
    (1, pdb.term('NaN'::float8)),
    (2, pdb.term('Infinity'::float8)),
    (3, pdb.term('-Infinity'::float8));
DO $$
DECLARE
    saved record;
    got int[];
BEGIN
    FOR saved IN SELECT * FROM issue_7237_saved_queries LOOP
        EXECUTE 'SELECT ARRAY(SELECT id FROM issue_7237_t WHERE d @@@ $1 ORDER BY id)'
            INTO got USING saved.query;
        IF got IS DISTINCT FROM ARRAY[saved.expected_id] THEN
            RAISE EXCEPTION 'stored query: got %, want %', got, ARRAY[saved.expected_id];
        END IF;
        EXECUTE 'SELECT ARRAY(SELECT id FROM issue_7237_t WHERE d @@@ $1 ORDER BY id)'
            INTO got USING saved.query::text::pdb.query;
        IF got IS DISTINCT FROM ARRAY[saved.expected_id] THEN
            RAISE EXCEPTION 'text round trip: got %, want %', got, ARRAY[saved.expected_id];
        END IF;
    END LOOP;
END $$;
DROP TABLE issue_7237_saved_queries;

-- A generic plan carries the constant as a parameter
SET plan_cache_mode = force_generic_plan;
PREPARE issue_7237_q(float8) AS
SELECT id FROM issue_7237_t WHERE issue_7237_t @@@ pdb.all() AND d = $1;
EXECUTE issue_7237_q('NaN');
EXECUTE issue_7237_q('Infinity');
EXECUTE issue_7237_q(1.5);
DEALLOCATE issue_7237_q;
RESET plan_cache_mode;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
DROP FUNCTION issue_7237_mismatches(text);
DROP TABLE issue_7237_imm;
DROP TABLE issue_7237_t;
DROP TABLE issue_7237_plain;
