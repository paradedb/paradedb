-- Setup
CREATE TABLE persons (id SERIAL PRIMARY KEY, name TEXT);
CREATE TABLE jobs (id SERIAL PRIMARY KEY, person_id INT REFERENCES persons(id), title TEXT);

INSERT INTO persons VALUES (1,'Alice'), (2,'Bob'), (3,'Charlie');
INSERT INTO jobs VALUES
  (1, 1, 'Senior Developer'),  -- Alice
  (2, 1, 'Tech Lead'),         -- Alice
  (3, 2, 'Developer'),         -- Bob  ← matches "developer"
  (4, 2, 'Recruiter'),         -- Bob  ← should exclude Bob
  (5, 3, 'Recruiter');         -- Charlie

CREATE INDEX jobs_bm25 ON jobs USING bm25 (id, person_id, title) WITH (key_field='id');
CREATE INDEX persons_bm25 ON persons USING bm25 (id, name) WITH (key_field='id', text_fields='{"name": {"fast": true}}');



-- Extra setup for the second query
CREATE TABLE profiles_positions (
    id SERIAL PRIMARY KEY,
    profile_id INT,
    group_id TEXT,
    deleted_at TIMESTAMP,
    title TEXT
);
INSERT INTO profiles_positions (profile_id, group_id, deleted_at, title) VALUES
  (1, 'c624233d-2c2c-43ab-976e-0847bfd58387', NULL, 'Senior Developer'),
  (1, 'c624233d-2c2c-43ab-976e-0847bfd58387', NULL, 'Tech Lead'),
  (2, 'c624233d-2c2c-43ab-976e-0847bfd58387', NULL, 'Developer'),
  (2, 'c624233d-2c2c-43ab-976e-0847bfd58387', NULL, 'Recruiter'),
  (3, 'c624233d-2c2c-43ab-976e-0847bfd58387', NULL, 'Recruiter');

CREATE INDEX profiles_positions_bm25 ON profiles_positions USING bm25 (id, profile_id, group_id, deleted_at, title) WITH (key_field='id');

-- Query 1
SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF)
SELECT DISTINCT p.id, p.name
FROM persons p
JOIN jobs j ON j.person_id = p.id
WHERE j.id @@@ paradedb.parse('title:developer')
  AND p.id NOT IN (
    SELECT j2.person_id
    FROM jobs j2
    WHERE j2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY p.id
LIMIT 50;

SELECT DISTINCT p.id, p.name
FROM persons p
JOIN jobs j ON j.person_id = p.id
WHERE j.id @@@ paradedb.parse('title:developer')
  AND p.id NOT IN (
    SELECT j2.person_id
    FROM jobs j2
    WHERE j2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY p.id
LIMIT 50;

SET paradedb.enable_join_custom_scan = off;

EXPLAIN (COSTS OFF)
SELECT DISTINCT p.id, p.name
FROM persons p
JOIN jobs j ON j.person_id = p.id
WHERE j.id @@@ paradedb.parse('title:developer')
  AND p.id NOT IN (
    SELECT j2.person_id
    FROM jobs j2
    WHERE j2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY p.id
LIMIT 50;

SELECT DISTINCT p.id, p.name
FROM persons p
JOIN jobs j ON j.person_id = p.id
WHERE j.id @@@ paradedb.parse('title:developer')
  AND p.id NOT IN (
    SELECT j2.person_id
    FROM jobs j2
    WHERE j2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY p.id
LIMIT 50;

-- Query 2
SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF)
SELECT DISTINCT profile_id
FROM profiles_positions pp
WHERE deleted_at IS NULL
  AND group_id = 'c624233d-2c2c-43ab-976e-0847bfd58387'
  AND id @@@ paradedb.parse('title:developer')
  AND NOT EXISTS (
    SELECT 1
    FROM profiles_positions pp2
    WHERE pp2.deleted_at IS NULL
      AND pp2.group_id = pp.group_id
      AND pp2.profile_id = pp.profile_id
      AND pp2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY profile_id
LIMIT 50;

SELECT DISTINCT profile_id
FROM profiles_positions pp
WHERE deleted_at IS NULL
  AND group_id = 'c624233d-2c2c-43ab-976e-0847bfd58387'
  AND id @@@ paradedb.parse('title:developer')
  AND NOT EXISTS (
    SELECT 1
    FROM profiles_positions pp2
    WHERE pp2.deleted_at IS NULL
      AND pp2.group_id = pp.group_id
      AND pp2.profile_id = pp.profile_id
      AND pp2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY profile_id
LIMIT 50;

SET paradedb.enable_join_custom_scan = off;

EXPLAIN (COSTS OFF)
SELECT DISTINCT profile_id
FROM profiles_positions pp
WHERE deleted_at IS NULL
  AND group_id = 'c624233d-2c2c-43ab-976e-0847bfd58387'
  AND id @@@ paradedb.parse('title:developer')
  AND NOT EXISTS (
    SELECT 1
    FROM profiles_positions pp2
    WHERE pp2.deleted_at IS NULL
      AND pp2.group_id = pp.group_id
      AND pp2.profile_id = pp.profile_id
      AND pp2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY profile_id
LIMIT 50;

SELECT DISTINCT profile_id
FROM profiles_positions pp
WHERE deleted_at IS NULL
  AND group_id = 'c624233d-2c2c-43ab-976e-0847bfd58387'
  AND id @@@ paradedb.parse('title:developer')
  AND NOT EXISTS (
    SELECT 1
    FROM profiles_positions pp2
    WHERE pp2.deleted_at IS NULL
      AND pp2.group_id = pp.group_id
      AND pp2.profile_id = pp.profile_id
      AND pp2.id @@@ paradedb.parse('title:recruiter')
  )
ORDER BY profile_id
LIMIT 50;

-- Teardown
DROP TABLE persons CASCADE;
DROP TABLE jobs CASCADE;
DROP TABLE profiles_positions CASCADE;
