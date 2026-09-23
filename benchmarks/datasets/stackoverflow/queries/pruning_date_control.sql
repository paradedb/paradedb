-- Full observed date range: measures pruning overhead when dates exclude no non-null rows.

SELECT count(*)
FROM stackoverflow_posts
WHERE body ||| 'javascript'
  AND creation_date BETWEEN '{{ date_min }}' AND '{{ date_max }}';
