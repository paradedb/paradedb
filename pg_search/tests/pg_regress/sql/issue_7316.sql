CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.planner_warnings = 'off';

-- check_join_scan runs a query with Join Scan on and off: join_scan says Join Scan
-- planned it, matches_postgres that both runs gave the same rows or the same error.
CREATE FUNCTION check_join_scan(query text)
RETURNS TABLE (join_scan boolean, matches_postgres boolean, result text) AS $$
DECLARE
    line text;
    with_scan text;
    without_scan text;
    wrapped text := 'SELECT array_agg(t::text ORDER BY t::text)::text FROM (' || query || ') t';
BEGIN
    join_scan := false;
    FOR line IN EXECUTE 'EXPLAIN (COSTS OFF) ' || query LOOP
        IF line LIKE '%ParadeDB Join Scan%' THEN
            join_scan := true;
        END IF;
    END LOOP;
    BEGIN
        EXECUTE wrapped INTO with_scan;
    EXCEPTION WHEN OTHERS THEN
        with_scan := 'ERROR: ' || SQLERRM;
    END;
    PERFORM set_config('paradedb.enable_join_custom_scan', 'off', true);
    BEGIN
        EXECUTE wrapped INTO without_scan;
    EXCEPTION WHEN OTHERS THEN
        without_scan := 'ERROR: ' || SQLERRM;
    END;
    PERFORM set_config('paradedb.enable_join_custom_scan', 'on', true);
    matches_postgres := with_scan IS NOT DISTINCT FROM without_scan;
    result := with_scan;
    RETURN NEXT;
END
$$ LANGUAGE plpgsql;

CREATE TABLE issue_7316_a (id int PRIMARY KEY, f float8, r real, q int, t text, d date, b bool);
CREATE TABLE issue_7316_b (id int PRIMARY KEY, a_id int, g float8, n int, s text);
INSERT INTO issue_7316_a VALUES
    (1, 2.5,   1.1,  5,           'abc',   '2024-01-05', true),
    (2, -2.5,  2.2,  -2147483647, '',      '2024-01-03', false),
    (3, '-0',  0.3,  0,           'héllo', '2024-02-29', true),
    (4, 0,     -1.5, -1,          'zz',    '2000-01-01', NULL),
    (5, -1,    7.7,  7,           'lol',   '2024-01-09', false),
    (6, 'NaN', 2.0,  2,           'x',     '2024-01-07', true),
    (7, 1e308, 3.3,  3,           'y',     '2024-01-02', false),
    (8, 1,     1.5,  -2147483648, 'Zed',   '2024-01-02', true);
INSERT INTO issue_7316_b SELECT i, i, 1, 1, '' FROM generate_series(1, 8) i;
CREATE INDEX issue_7316_a_idx ON issue_7316_a USING paradedb (id, f, r, q, (t::pdb.literal), d, b);
CREATE INDEX issue_7316_b_idx ON issue_7316_b USING paradedb (id, a_id, g, n, (s::pdb.literal));

SET paradedb.enable_join_custom_scan = on;

-- round() in a Join Scan is evaluated by PostgreSQL
EXPLAIN (COSTS OFF)
SELECT a.id FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all() AND round(a.f * b.g) = 2 ORDER BY a.id LIMIT 10;

-- round, power, ln, log, sign and sqrt
SELECT * FROM check_join_scan($$SELECT a.id, round(a.f * b.g) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, power(a.f * b.g, 2) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id BETWEEN 1 AND 3 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, power(a.f * b.g, 2) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, power(a.f * b.g, 0.5) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, ln(a.f * b.g) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, ln(a.f * b.g), log(a.f * b.g) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id IN (1, 7) AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, sign(a.f * b.g), sqrt(abs(a.f * b.g)) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, sqrt(a.f * b.g) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE round(a.f * b.g) = 2 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- NULLIF, GREATEST and LEAST over floats, which have -0.0
SELECT * FROM check_join_scan($$SELECT a.id, nullif(a.f * b.g, 0) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, greatest(a.f * b.g, 0), least(a.f * b.g, 0) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- CONCAT of a float, boolean, date or integer
SELECT * FROM check_join_scan($$SELECT a.id, concat(a.t, b.s, a.f * b.g) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, concat(a.t, a.b), concat(a.t, a.d), concat(a.t, b.s, a.q) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- A real column in the select list and in a join filter
SELECT * FROM check_join_scan($$SELECT a.id, a.r * b.g, a.r * 3, a.r + a.r, a.r / 3 FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.r > 1.1, a.r = 1.1, a.r::float8 * 3, abs(a.r), round(a.r) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.r * b.g > 1.1 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.r > 1.1 AND b.g = 1 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- CASE with an operand and a boolean CASE are evaluated by PostgreSQL
SELECT * FROM check_join_scan($$SELECT a.id, CASE a.q WHEN 5 THEN 'five' WHEN 7 THEN 'seven' ELSE 'other' END, CASE a.b WHEN true THEN 1 ELSE 0 END FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, CASE a.f * b.g WHEN a.f THEN 'match' ELSE 'other' END FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, CASE WHEN a.q = 0 THEN false ELSE 100 / a.q > 0 END FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- ANY and ALL with each operator, -0.0 in = ANY and IN, and a real column
SELECT * FROM check_join_scan($$SELECT a.id, a.f > ANY(ARRAY[b.g, 100::float8]), a.f < ALL(ARRAY[b.g, 100::float8]) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.f = ANY(ARRAY[b.g, 2.5::float8]), a.f <> ALL(ARRAY[b.g, 2.5::float8]) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, (a.r * '-0'::real) = ANY(ARRAY[b.g::real - 1::real, 5::real]), (a.f * -1.0) IN (b.g - 1, 5::float8) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.r = ANY(ARRAY[(b.g * 2.2)::real, 1.5::real]) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- Integer abs and division at INT_MIN, remainders, and a zero divisor
SELECT * FROM check_join_scan($$SELECT a.id, abs(a.q) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, abs(a.q) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id < 8 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.q / -1 FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.q % -1, a.q / (b.n + 1) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id < 8 AND a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.q % (b.n - 1) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, a.q / (b.n - 1) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- Date arithmetic, alone and compared with a date
SELECT * FROM check_join_scan($$SELECT a.id, a.d + b.n, a.d - b.n, a.d - (a.d - b.n) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, (a.d + b.n) > a.d, (a.d - b.n) IN (a.d, '2000-01-01'::date) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- Text GREATEST, LEAST, NULLIF and = ANY under a collation
SELECT * FROM check_join_scan($$SELECT a.id, greatest(a.t COLLATE "und-x-icu", 'b'), least(a.t COLLATE "und-x-icu", 'B') FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, nullif(a.t COLLATE "und-x-icu", 'abc'), a.t = ANY(ARRAY['abc', 'ZED']) FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

-- Expressions that stay in the scan are unchanged
SELECT * FROM check_join_scan($$SELECT a.id, abs(a.f), ceil(a.f), floor(a.f), (a.f * b.g) < 0, coalesce(a.f, b.g), a.r > 1.1 FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);
SELECT * FROM check_join_scan($$SELECT a.id, length(a.t || b.s), substr(a.t || b.s, 2), repeat(a.t || b.s, 2), a.q = b.n FROM issue_7316_a a JOIN issue_7316_b b ON b.a_id = a.id WHERE a.id @@@ pdb.all() ORDER BY a.id LIMIT 10$$);

RESET paradedb.enable_join_custom_scan;

DROP TABLE issue_7316_b;
DROP TABLE issue_7316_a;
DROP FUNCTION check_join_scan(text);
