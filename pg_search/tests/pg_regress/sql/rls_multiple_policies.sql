CREATE EXTENSION IF NOT EXISTS pg_search;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_roles
        WHERE rolname = 'authenticated'
    ) THEN
        CREATE ROLE authenticated;
    END IF;
END
$$;

DO $$
DECLARE
    v_session_user name := session_user;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_auth_members m
        JOIN pg_roles r_member ON r_member.oid = m.member
        JOIN pg_roles r_role   ON r_role.oid   = m.roleid
        WHERE r_member.rolname = v_session_user
          AND r_role.rolname = 'authenticated'
    ) THEN
        EXECUTE format('GRANT authenticated TO %I', v_session_user);
    END IF;
END
$$;

SET client_min_messages TO warning;
DROP TABLE IF EXISTS documents CASCADE;
DROP TABLE IF EXISTS access_tags CASCADE;
DROP FUNCTION IF EXISTS check_org_access(uuid);
DROP FUNCTION IF EXISTS document_has_tags(bigint);
DROP FUNCTION IF EXISTS get_document_tag_ids(bigint);
RESET client_min_messages;

CREATE TABLE access_tags (
    id int PRIMARY KEY,
    org_id uuid NOT NULL
);

INSERT INTO access_tags VALUES
    (1, 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'),
    (2, 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb');

CREATE TABLE documents (
    id bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    org_id uuid NOT NULL DEFAULT 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    title text NOT NULL,
    tag_ids int[] DEFAULT '{}'
);

INSERT INTO documents (title, tag_ids) VALUES
    ('sheriff department los angeles', '{}'),
    ('cloud computing infrastructure', '{1}'),
    ('sheriff office county records', '{2}'),
    ('sheriff cross org incident report', '{}');

UPDATE documents
SET org_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
WHERE title = 'sheriff cross org incident report';

CREATE INDEX documents_bm25 ON documents
    USING bm25 (id, title) WITH (key_field=id);

GRANT SELECT ON documents TO authenticated;
GRANT SELECT ON access_tags TO authenticated;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents FORCE ROW LEVEL SECURITY;

CREATE FUNCTION check_org_access(uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT $1 = current_setting('request.jwt.claim.org_id')::uuid;
$$;

CREATE FUNCTION document_has_tags(bigint)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS(
        SELECT 1 FROM documents
        WHERE id = $1 AND array_length(tag_ids, 1) > 0
    );
$$;

CREATE FUNCTION get_document_tag_ids(bigint)
RETURNS int[] LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT tag_ids FROM documents WHERE id = $1;
$$;

-- First policy

CREATE POLICY org_access ON documents FOR SELECT TO authenticated
    USING (check_org_access(org_id));

BEGIN;
SET LOCAL ROLE authenticated;
SET LOCAL request.jwt.claim.org_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

SELECT id, title, pdb.score(id) AS score,
       pdb.snippet(title, start_tag => '<b>', end_tag => '</b>') AS snippet
FROM documents
WHERE title ||| 'sheriff'
ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id)
FROM documents
WHERE title ||| 'sheriff';
COMMIT;

-- Second policy
CREATE POLICY tag_access ON documents AS RESTRICTIVE
    FOR SELECT TO authenticated
    USING (
        NOT document_has_tags(id)
        OR EXISTS (
            SELECT 1
            FROM unnest(get_document_tag_ids(documents.id)) AS tag_id
            JOIN access_tags ON access_tags.id = tag_id
            WHERE check_org_access(access_tags.org_id)
        )
    );

BEGIN;
SET LOCAL ROLE authenticated;
SET LOCAL request.jwt.claim.org_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id)
FROM documents
WHERE title ||| 'sheriff';

SELECT id
FROM documents
WHERE title ||| 'sheriff'
ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id)
FROM documents
WHERE title ||| 'sheriff'
ORDER BY pdb.score(id) DESC, id
LIMIT 2;

SELECT id
FROM documents
WHERE title ||| 'sheriff'
ORDER BY pdb.score(id) DESC, id
LIMIT 2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, pdb.score(id) AS score,
       pdb.snippet(title, start_tag => '<b>', end_tag => '</b>') AS snippet
FROM documents
WHERE title ||| 'sheriff'
ORDER BY id;

SELECT id, title, pdb.score(id) AS score,
       pdb.snippet(title, start_tag => '<b>', end_tag => '</b>') AS snippet
FROM documents
WHERE title ||| 'sheriff'
ORDER BY id;

SET LOCAL request.jwt.claim.org_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

SELECT id, title
FROM documents
WHERE title ||| 'sheriff'
ORDER BY id;
COMMIT;

REVOKE SELECT ON documents FROM authenticated;
REVOKE SELECT ON access_tags FROM authenticated;
REVOKE authenticated FROM current_user;

DROP POLICY org_access ON documents;
DROP POLICY tag_access ON documents;
DROP TABLE documents CASCADE;
DROP TABLE access_tags CASCADE;
DROP FUNCTION IF EXISTS check_org_access(uuid);
DROP FUNCTION IF EXISTS document_has_tags(bigint);
DROP FUNCTION IF EXISTS get_document_tag_ids(bigint);
