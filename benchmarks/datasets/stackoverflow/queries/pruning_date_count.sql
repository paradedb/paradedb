-- Text search within approximately 1% of the observed date distribution.

SELECT count(*)
FROM stackoverflow_posts
WHERE body ||| 'javascript'
  AND creation_date >= '{{ date_50 }}' AND creation_date < '{{ date_51 }}';
