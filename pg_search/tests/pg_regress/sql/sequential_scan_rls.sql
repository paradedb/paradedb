CREATE EXTENSION IF NOT EXISTS pg_search;

-- UPDATE ... RETURNING must evaluate search policies against the prospective row.
DROP TABLE IF EXISTS rls_native_new_row CASCADE;
DROP TABLE IF EXISTS rls_pdb_new_row CASCADE;
DROP ROLE IF EXISTS rls_new_row_user;

CREATE ROLE rls_new_row_user;

CREATE TABLE rls_native_new_row (
    id BIGINT PRIMARY KEY,
    body TEXT NOT NULL,
    note TEXT NOT NULL
);

CREATE TABLE rls_pdb_new_row (
    id BIGINT PRIMARY KEY,
    body TEXT NOT NULL,
    note TEXT NOT NULL
);

INSERT INTO rls_native_new_row VALUES
    (1, 'allowed', 'before'),
    (2, 'allowed', 'before');

INSERT INTO rls_pdb_new_row TABLE rls_native_new_row;

CREATE INDEX rls_pdb_new_row_idx ON rls_pdb_new_row
USING paradedb (id, body) WITH (key_field = id);

GRANT SELECT, UPDATE ON rls_native_new_row TO rls_new_row_user;
GRANT SELECT, UPDATE ON rls_pdb_new_row TO rls_new_row_user;

ALTER TABLE rls_native_new_row ENABLE ROW LEVEL SECURITY;
ALTER TABLE rls_pdb_new_row ENABLE ROW LEVEL SECURITY;

CREATE POLICY native_select_policy ON rls_native_new_row
FOR SELECT TO rls_new_row_user
USING (body = 'allowed');

CREATE POLICY native_update_policy ON rls_native_new_row
FOR UPDATE TO rls_new_row_user
USING (true)
WITH CHECK (true);

CREATE POLICY pdb_select_policy ON rls_pdb_new_row
FOR SELECT TO rls_new_row_user
USING (id @@@ paradedb.term('body', 'allowed'));

CREATE POLICY pdb_update_policy ON rls_pdb_new_row
FOR UPDATE TO rls_new_row_user
USING (true)
WITH CHECK (true);

SET client_min_messages = ERROR;
SET ROLE rls_new_row_user;

UPDATE rls_native_new_row
SET body = 'denied'
WHERE id = 1
RETURNING id, body, note;

UPDATE rls_native_new_row
SET note = 'after'
WHERE id = 2
RETURNING id, body, note;

UPDATE rls_pdb_new_row
SET body = 'denied'
WHERE id = 1
RETURNING id, body, note;

UPDATE rls_pdb_new_row
SET note = 'after'
WHERE id = 2
RETURNING id, body, note;

RESET ROLE;
RESET client_min_messages;

TABLE rls_native_new_row;
TABLE rls_pdb_new_row;

-- A keyless partial index must not hide existing or NEW rows from the search policy.
DROP INDEX rls_pdb_new_row_idx;
CREATE INDEX rls_pdb_new_row_idx ON rls_pdb_new_row
USING paradedb (id, body) WHERE note = 'before';

SET client_min_messages = ERROR;
SET ROLE rls_new_row_user;
SELECT id FROM rls_native_new_row ORDER BY id;
SELECT id FROM rls_pdb_new_row ORDER BY id;

UPDATE rls_native_new_row SET note = 'outside' WHERE id = 1 RETURNING id, body, note;
UPDATE rls_pdb_new_row SET note = 'outside' WHERE id = 1 RETURNING id, body, note;
UPDATE rls_native_new_row SET note = 'before' WHERE id = 2 RETURNING id, body, note;
UPDATE rls_pdb_new_row SET note = 'before' WHERE id = 2 RETURNING id, body, note;
UPDATE rls_native_new_row SET body = 'denied' WHERE id = 1 RETURNING id, body, note;
UPDATE rls_pdb_new_row SET body = 'denied' WHERE id = 1 RETURNING id, body, note;

RESET ROLE;
RESET client_min_messages;
TABLE rls_native_new_row;
TABLE rls_pdb_new_row;

DROP TABLE rls_native_new_row;
DROP TABLE rls_pdb_new_row;
DROP ROLE rls_new_row_user;
