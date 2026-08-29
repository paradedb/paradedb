-- Regression test for #6022: preserve paradedb.score() through filtered Top-K
-- plans regardless of whether PostgreSQL uses a join, semi-join, or SubPlan.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_6022_filters CASCADE;
DROP TABLE IF EXISTS issue_6022_documents CASCADE;

CREATE TABLE issue_6022_documents (
    id bigint PRIMARY KEY,
    body text NOT NULL
);

CREATE TABLE issue_6022_filters (
    document_id bigint NOT NULL REFERENCES issue_6022_documents(id),
    selected boolean NOT NULL
);

INSERT INTO issue_6022_documents (id, body) VALUES
    (1, 'keyboard keyboard keyboard'),
    (2, 'keyboard keyboard'),
    (3, 'keyboard'),
    (4, 'keyboard mouse'),
    (5, 'keyboard monitor'),
    (6, 'keyboard'),
    (7, 'mouse only'),
    (8, 'monitor only');

INSERT INTO issue_6022_filters (document_id, selected)
SELECT id, id BETWEEN 2 AND 6
FROM issue_6022_documents;

CREATE INDEX issue_6022_documents_bm25 ON issue_6022_documents
USING paradedb (id, body) WITH (key_field = 'id');

ANALYZE issue_6022_documents;
ANALYZE issue_6022_filters;

-- Single-relation control: the no-placeholder fast path remains valid.
CREATE TEMP TABLE issue_6022_expected_scores AS
SELECT id, paradedb.score(id) AS score
FROM issue_6022_documents
WHERE id @@@ paradedb.match('body', 'keyboard')
ORDER BY paradedb.score(id) DESC, id
LIMIT 6;

SELECT array_agg(id ORDER BY score DESC, id) AS ids,
       bool_and(score > 0) AS scores_computed
FROM (
    SELECT *
    FROM issue_6022_expected_scores
    ORDER BY score DESC, id
    LIMIT 3
) AS top_three;

-- Explicit derived INNER JOIN.
CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
INNER JOIN (
    SELECT DISTINCT document_id
    FROM issue_6022_filters
    WHERE selected
) AS f ON f.document_id = d.id
WHERE d.id @@@ paradedb.match('body', 'keyboard')
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

-- Pull-up eligible IN filter.
CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
WHERE d.id @@@ paradedb.match('body', 'keyboard')
  AND d.id IN (
      SELECT document_id
      FROM issue_6022_filters
      WHERE selected
  )
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

-- LIMIT ALL keeps the equivalent IN filter as a sublink/subplan.
CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
WHERE d.id @@@ paradedb.match('body', 'keyboard')
  AND d.id IN (
      SELECT document_id
      FROM issue_6022_filters
      WHERE selected
      LIMIT ALL
  )
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

-- Correlated EXISTS may be represented as a pulled-up semi-join.
CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
WHERE d.id @@@ paradedb.match('body', 'keyboard')
  AND EXISTS (
      SELECT 1
      FROM issue_6022_filters AS f
      WHERE f.document_id = d.id
        AND f.selected
  )
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

-- Increase related-table cardinality without changing the qualifying document
-- set, then repeat the vulnerable retained-sublink and semi-join forms.
INSERT INTO issue_6022_filters (document_id, selected)
SELECT d.id, true
FROM issue_6022_documents AS d
CROSS JOIN generate_series(1, 100)
WHERE d.id BETWEEN 2 AND 6;

ANALYZE issue_6022_filters;

CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
WHERE d.id @@@ paradedb.match('body', 'keyboard')
  AND d.id IN (
      SELECT document_id
      FROM issue_6022_filters
      WHERE selected
      LIMIT ALL
  )
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

CREATE TEMP TABLE issue_6022_actual AS
SELECT d.id, paradedb.score(d.id) AS score
FROM issue_6022_documents AS d
WHERE d.id @@@ paradedb.match('body', 'keyboard')
  AND EXISTS (
      SELECT 1
      FROM issue_6022_filters AS f
      WHERE f.document_id = d.id
        AND f.selected
  )
ORDER BY paradedb.score(d.id) DESC, d.id
LIMIT 3;

SELECT array_agg(a.id ORDER BY a.score DESC, a.id) AS ids,
       bool_and(a.score = e.score) AS scores_match
FROM issue_6022_actual AS a
JOIN issue_6022_expected_scores AS e USING (id);

DROP TABLE issue_6022_actual;

DROP TABLE IF EXISTS issue_6022_filters CASCADE;
DROP TABLE IF EXISTS issue_6022_documents CASCADE;
