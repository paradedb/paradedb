-- Tests that a `date`, `timestamp` or `timestamptz` constant after year 9999 or BC
-- can be pushed down to the index and searched (issue #6731).
CREATE EXTENSION IF NOT EXISTS pg_search;
SET TimeZone = 'UTC';

CREATE TABLE issue_6731 (id serial PRIMARY KEY, d date, t timestamp, tz timestamptz);
INSERT INTO issue_6731 (d, t, tz) VALUES
    ('2024-01-01', '2024-01-01', '2024-01-01'),
    ('9999-12-31', '9999-12-31 23:59:59', '9999-12-31 23:59:59'),
    ('10000-01-01', '10000-01-01', '10000-01-01'),
    ('0044-03-15 BC', '0044-03-15 12:34:56.789 BC', '0044-03-15 12:34:56.789 BC');
CREATE INDEX issue_6731_idx ON issue_6731 USING paradedb (id, d, t, tz);

EXPLAIN (COSTS OFF)
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d < '10000-01-01'::date ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d < '10000-01-01'::date ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d >= '10000-01-01'::date ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d = '10000-01-01'::date ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d > '0001-01-01 BC'::date ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND d = '0044-03-15 BC'::date ORDER BY id;

SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND t > '0001-01-01 BC'::timestamp ORDER BY id;
SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND t < '10000-01-01'::timestamp ORDER BY id;
SELECT id FROM issue_6731
WHERE id @@@ pdb.all() AND t = '0044-03-15 12:34:56.789 BC'::timestamp ORDER BY id;

SELECT id FROM issue_6731 WHERE id @@@ pdb.all() AND tz >= '10000-01-01'::timestamptz ORDER BY id;
SELECT id FROM issue_6731
WHERE id @@@ pdb.all() AND tz BETWEEN '0100-01-01 BC'::timestamptz AND '0001-01-01'::timestamptz
ORDER BY id;

RESET TimeZone;
DROP TABLE issue_6731;
