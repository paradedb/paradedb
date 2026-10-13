-- Regression coverage for issue #6699.
--
-- Search predicates (`|||`, `&&&`, `@@@`) inside an aggregate FILTER clause
-- must be pushed down to the DataFusion aggregate scan as tagged scan columns,
-- rather than falling back to a native GroupAggregate that evaluates the
-- search operator row by row.

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE issue_6699_authors (
    author_id INTEGER PRIMARY KEY,
    name TEXT,
    bio TEXT
);

CREATE TABLE issue_6699_articles (
    id SERIAL PRIMARY KEY,
    author_id INTEGER,
    category TEXT,
    body TEXT,
    created_at DATE
);

INSERT INTO issue_6699_authors (author_id, name, bio) VALUES
    (1, 'alice', 'database engineer'),
    (2, 'bob', 'frontend developer'),
    (3, 'carol', 'writes about database internals'),
    (4, 'dave', 'gardening enthusiast');

INSERT INTO issue_6699_articles (author_id, category, body, created_at) VALUES
    (1, 'tutorial', 'intro to database indexing', '2024-01-01'),
    (1, 'tutorial', 'database query planning', '2024-02-01'),
    (1, 'news', 'database release notes', '2024-03-01'),
    (2, 'tutorial', 'css layout tutorial', '2024-01-15'),
    (2, 'tutorial', 'database for frontend developers', '2024-04-01'),
    (3, 'opinion', 'why I love search engines', '2024-05-01'),
    (3, 'tutorial', 'search engine basics', '2024-02-15'),
    (4, 'news', 'gardening in winter', '2024-06-01'),
    (4, 'tutorial', 'database of plants', '2024-06-15');

CREATE INDEX issue_6699_authors_idx ON issue_6699_authors
USING paradedb (author_id, name, bio);

CREATE INDEX issue_6699_articles_idx ON issue_6699_articles
USING paradedb (id, author_id, (category::pdb.literal), body, created_at);

-- The query from the issue: a filtered COUNT that is projected and used for
-- ranking. The FILTER's search predicate also appears in WHERE, so its tag can
-- be reused. The plan should be a ParadeDB Aggregate Scan whose DataFusion
-- aggregate filter references a tag column rather than a native Aggregate.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT
    a.author_id,
    COUNT(*) FILTER (
        WHERE a.category = 'tutorial'
          AND a.body ||| 'database'
    ) AS tutorial_count,
    MAX(a.created_at) AS latest_date
FROM issue_6699_articles a
JOIN issue_6699_authors u ON a.author_id = u.author_id
WHERE a.body ||| 'database'
   OR u.bio ||| 'database'
GROUP BY a.author_id
ORDER BY
    COUNT(*) FILTER (
        WHERE a.category = 'tutorial'
          AND a.body ||| 'database'
    ) DESC,
    MAX(a.created_at) DESC NULLS LAST,
    a.author_id
LIMIT 15 OFFSET 0;

SELECT
    a.author_id,
    COUNT(*) FILTER (
        WHERE a.category = 'tutorial'
          AND a.body ||| 'database'
    ) AS tutorial_count,
    MAX(a.created_at) AS latest_date
FROM issue_6699_articles a
JOIN issue_6699_authors u ON a.author_id = u.author_id
WHERE a.body ||| 'database'
   OR u.bio ||| 'database'
GROUP BY a.author_id
ORDER BY
    COUNT(*) FILTER (
        WHERE a.category = 'tutorial'
          AND a.body ||| 'database'
    ) DESC,
    MAX(a.created_at) DESC NULLS LAST,
    a.author_id
LIMIT 15 OFFSET 0;

-- FILTER search predicates that do not appear in WHERE, on both sides of the
-- join and with each search operator, each need their own tag column.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT a.author_id,
       COUNT(*) FILTER (WHERE a.body ||| 'search') AS search_count,
       COUNT(*) FILTER (WHERE a.body &&& 'database query') AS planning_count,
       COUNT(*) FILTER (WHERE a.body @@@ pdb.term('engine')) AS engine_count,
       COUNT(*) FILTER (WHERE NOT (u.bio ||| 'database')) AS other_bio_count
FROM issue_6699_articles a
JOIN issue_6699_authors u ON a.author_id = u.author_id
WHERE a.id @@@ pdb.all()
GROUP BY a.author_id;

SELECT a.author_id,
       COUNT(*) FILTER (WHERE a.body ||| 'search') AS search_count,
       COUNT(*) FILTER (WHERE a.body &&& 'database query') AS planning_count,
       COUNT(*) FILTER (WHERE a.body @@@ pdb.term('engine')) AS engine_count,
       COUNT(*) FILTER (WHERE NOT (u.bio ||| 'database')) AS other_bio_count
FROM issue_6699_articles a
JOIN issue_6699_authors u ON a.author_id = u.author_id
WHERE a.id @@@ pdb.all()
GROUP BY a.author_id
ORDER BY a.author_id;

DROP TABLE issue_6699_articles;
DROP TABLE issue_6699_authors;
