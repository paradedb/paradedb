\i common/common_setup.sql

-- the original issue repro
CREATE OR REPLACE FUNCTION get_day_of_week(d date)
RETURNS integer
LANGUAGE sql
IMMUTABLE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT CASE
        WHEN d IS NULL THEN NULL
        ELSE EXTRACT(ISODOW FROM d)::int
    END;
$$;

CREATE OR REPLACE FUNCTION get_day_of_week_plus_one(d date)
RETURNS integer
LANGUAGE sql
IMMUTABLE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT CASE
        WHEN d IS NULL THEN NULL
        ELSE EXTRACT(ISODOW FROM d)::int
    END;
$$;

DROP TABLE IF EXISTS dates;
CREATE TABLE dates (id SERIAL PRIMARY KEY, d date);
INSERT INTO dates (d) VALUES ('2025-01-01');
CREATE INDEX idx_dates ON dates USING bm25 (id, (get_day_of_week(d)::pdb.alias('d'))) with (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM dates WHERE get_day_of_week(d) = 3 AND id @@@ pdb.all();

SELECT * FROM dates WHERE get_day_of_week(d) = 3 AND id @@@ pdb.all();

-- make sure anything that's not this exact expression does not get pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM dates WHERE get_day_of_week_plus_one(d) = 3 AND id @@@ pdb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM dates WHERE d = '2025-01-01' AND id @@@ pdb.all();

DROP TABLE dates;

-- Verify that text/json types cannot be cast to pdb.alias
DO $$
DECLARE
    t text;
    typelist text[] := ARRAY['text', 'varchar', 'json', 'jsonb', 'text[]', 'varchar[]'];
BEGIN
    FOREACH t IN ARRAY typelist LOOP
        EXECUTE format('
            CREATE TABLE alias_test (
                id SERIAL PRIMARY KEY,
                col %s
            );', t);

        BEGIN
            EXECUTE '
                CREATE INDEX idx_alias_test ON alias_test
                USING bm25 (id, (col::pdb.alias(''mycol'')))
                WITH (key_field = ''id'')';
        EXCEPTION
            WHEN OTHERS THEN
                RAISE WARNING '%', SQLERRM;
        END;

        EXECUTE 'DROP TABLE alias_test';
    END LOOP;
END $$;

-- Verify that other types can
CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col smallint
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col integer
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col bigint
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col oid
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col float4
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col float8
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col numeric
);
INSERT INTO alias_test (col) VALUES (1);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col boolean
);
INSERT INTO alias_test (col) VALUES (true);
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ 'true';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col date
);
INSERT INTO alias_test (col) VALUES ('2025-01-01');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col time
);
INSERT INTO alias_test (col) VALUES ('00:00:00');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col timestamp
);
INSERT INTO alias_test (col) VALUES ('2025-01-01 00:00:00');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col timestamp with time zone
);
INSERT INTO alias_test (col) VALUES ('2025-01-01 00:00:00+00');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col time with time zone
);
INSERT INTO alias_test (col) VALUES ('00:00:00+00');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col smallint[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col integer[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col bigint[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col oid[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col float4[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col float8[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col numeric[]
);
INSERT INTO alias_test (col) VALUES ('{1, 2, 3}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ '1';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col boolean[]
);
INSERT INTO alias_test (col) VALUES ('{true, false, true}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
SELECT * FROM alias_test WHERE col::pdb.alias('mycol') @@@ 'true';
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col date[]
);
INSERT INTO alias_test (col) VALUES ('{2025-01-01, 2025-01-02, 2025-01-03}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col timestamp[]
);
INSERT INTO alias_test (col) VALUES ('{2025-01-01 00:00:00, 2025-01-02 00:00:00, 2025-01-03 00:00:00}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;

CREATE TABLE alias_test (
    id SERIAL PRIMARY KEY,
    col timestamp with time zone[]
);
INSERT INTO alias_test (col) VALUES ('{2025-01-01 00:00:00+00, 2025-01-02 00:00:00+00, 2025-01-03 00:00:00+00}');
CREATE INDEX idx_alias_test ON alias_test USING bm25 (id, (col::pdb.alias('mycol'))) WITH (key_field = 'id');
DROP TABLE alias_test;
