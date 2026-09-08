SET client_min_messages = error;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_custom_scan = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;

CREATE TABLE returning_current (id int PRIMARY KEY, body text NOT NULL, note text)
WITH (fillfactor = 50);
INSERT INTO returning_current VALUES (1, 'alpha', 'before');
CREATE INDEX returning_current_idx ON returning_current USING paradedb (id, body);

BEGIN;
UPDATE returning_current SET note = 'after' WHERE id = 1
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
ROLLBACK;

BEGIN;
INSERT INTO returning_current VALUES (2, 'alpha', 'new')
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
ROLLBACK;

BEGIN;
SAVEPOINT returning_subxid;
UPDATE returning_current SET body = 'beta' WHERE id = 1
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
INSERT INTO returning_current VALUES (2, 'alpha', 'new'), (3, 'beta', 'new')
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
RELEASE SAVEPOINT returning_subxid;

UPDATE returning_current SET body = 'alpha'
WHERE id = 1 AND body === 'beta'
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
SELECT id, body = 'alpha' AS native, body === 'alpha' AS paradedb
FROM returning_current ORDER BY id;
DELETE FROM returning_current WHERE id = 2 AND body === 'alpha'
RETURNING id, body = 'alpha' AS native, body === 'alpha' AS paradedb;
ROLLBACK;

DROP TABLE returning_current;

CREATE TABLE returning_old_new (id int PRIMARY KEY, body text NOT NULL, covered boolean NOT NULL);
INSERT INTO returning_old_new
SELECT id, CASE WHEN id % 2 = 1 THEN 'alpha' ELSE 'beta' END, id IN (3, 4, 7, 8)
FROM generate_series(1, 8) AS id;
CREATE INDEX returning_old_new_idx ON returning_old_new USING paradedb (id, body) WHERE covered;

DO $$
DECLARE
    matches boolean;
BEGIN
    IF current_setting('server_version_num')::int >= 180000 THEN
        WITH updated AS (
            UPDATE returning_old_new
            SET body = CASE body WHEN 'alpha' THEN 'beta' ELSE 'alpha' END, covered = id > 4
            RETURNING old.body = 'alpha' AS native_old, old.body === 'alpha' AS paradedb_old,
                      new.body = 'alpha' AS native_new, new.body === 'alpha' AS paradedb_new
        )
        SELECT count(*) = 8 AND bool_and(
            native_old IS NOT DISTINCT FROM paradedb_old
            AND native_new IS NOT DISTINCT FROM paradedb_new
        ) INTO matches FROM updated;
        ASSERT matches, 'RETURNING OLD/NEW must match before and after changing index coverage';
    END IF;
END;
$$;

DROP TABLE returning_old_new;
