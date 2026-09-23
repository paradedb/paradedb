-- Deterministic Top K within the most recent approximately 10% of dates.

SELECT id, title, creation_date
FROM stackoverflow_posts
WHERE body ||| 'javascript'
  AND creation_date >= '{{ date_90 }}'
ORDER BY creation_date DESC, id ASC
LIMIT 10;
