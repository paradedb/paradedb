BEGIN;

CREATE TABLE wikipedia_articles ( url TEXT, title TEXT, body TEXT );

CREATE TEMPORARY TABLE temp_json ( j JSONB ) ON COMMIT DROP;
COPY temp_json FROM '/Users/suriya-retake/Documents/paradedb/benchmark/wiki-articles-1000.json' CSV QUOTE E'\x01' DELIMITER E'\x02';

INSERT INTO wikipedia_articles ("url", "title", "body")

SELECT values->>'url' AS url,
       values->>'title' AS title,
       values->>'body' AS body
FROM   (SELECT j AS values from temp_json) A;

COMMIT;