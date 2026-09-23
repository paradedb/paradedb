-- Aggregate within approximately 10% of the observed date distribution.

SELECT post_type_id, count(*)
FROM stackoverflow_posts
WHERE id @@@ pdb.all()
  AND creation_date >= '{{ date_50 }}' AND creation_date < '{{ date_60 }}'
GROUP BY post_type_id
ORDER BY post_type_id;
