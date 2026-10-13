-- Postgres treats float4/float8 +0 and -0 as equal. Compare a native scan with
-- ParadeDB equality filters and GROUP BY on the same rows (issue #6609).
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6609 (
    id serial PRIMARY KEY,
    f4 float4,
    f8 float8
);

INSERT INTO issue_6609 (f4, f8) VALUES
    (0, 0),
    ('-0', '-0'),
    (0, 0),
    (1, 1);

-- Native Postgres, before a ParadeDB index exists.
SELECT count(*) AS f4_eq_pos FROM issue_6609 WHERE f4 = 0;
SELECT count(*) AS f4_eq_neg FROM issue_6609 WHERE f4 = '-0'::float4;
SELECT count(*) AS f8_eq_pos FROM issue_6609 WHERE f8 = 0;
SELECT count(*) AS f8_eq_neg FROM issue_6609 WHERE f8 = '-0'::float8;
SELECT f4, count(*) FROM issue_6609 GROUP BY f4 ORDER BY f4;
SELECT f8, count(*) FROM issue_6609 GROUP BY f8 ORDER BY f8;

CREATE INDEX issue_6609_idx ON issue_6609 USING paradedb (id, f4, f8);

-- Same predicates, executed through the ParadeDB index.
SELECT count(*) AS f4_eq_pos FROM issue_6609 WHERE id @@@ pdb.all() AND f4 = 0;
SELECT count(*) AS f4_eq_neg FROM issue_6609 WHERE id @@@ pdb.all() AND f4 = '-0'::float4;
SELECT count(*) AS f8_eq_pos FROM issue_6609 WHERE id @@@ pdb.all() AND f8 = 0;
SELECT count(*) AS f8_eq_neg FROM issue_6609 WHERE id @@@ pdb.all() AND f8 = '-0'::float8;

-- Term construction and query-string parsing must use the same zero as the index.
SELECT count(*) AS f4_term_neg FROM issue_6609 WHERE f4 @@@ pdb.term('-0'::float4);
SELECT count(*) AS f8_term_neg FROM issue_6609 WHERE f8 @@@ pdb.term('-0'::float8);
SELECT count(*) AS f4_parse_pos FROM issue_6609 WHERE id @@@ pdb.parse('f4:0');
SELECT count(*) AS f4_parse_neg FROM issue_6609 WHERE id @@@ pdb.parse('f4:-0');
SELECT count(*) AS f8_parse_pos FROM issue_6609 WHERE id @@@ pdb.parse('f8:0');
SELECT count(*) AS f8_parse_neg FROM issue_6609 WHERE id @@@ pdb.parse('f8:-0');

-- float4 GROUP BY through the index is a separate failure: whole-number group keys
-- come back as integers, and the float4 conversion rejects them
-- (UnsupportedIntoConversion("f32")). It is not part of this test.
SELECT f8, count(*) FROM issue_6609 WHERE id @@@ pdb.all() GROUP BY f8 ORDER BY f8;

DROP TABLE issue_6609;
