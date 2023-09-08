-- This script is inteneded to be run after the temp_json table has been filled.

BEGIN;

CREATE TABLE wikipedia_articles ( url TEXT, title TEXT, body TEXT );

-- This is executed directly from the command line with the -c option
-- CREATE TEMPORARY TABLE temp_json ( j JSONB ) ON COMMIT DROP;
-- COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';

INSERT INTO wikipedia_articles ("url", "title", "body")

SELECT values->>'url' AS url,
       values->>'title' AS title,
       values->>'body' AS body
FROM   (SELECT j AS values from temp_json) A;

DROP TABLE temp_json;

COMMIT;