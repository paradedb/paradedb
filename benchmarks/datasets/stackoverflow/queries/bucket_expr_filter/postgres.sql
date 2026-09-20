SELECT date_trunc('year', creation_date) as year, COUNT(*)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'javascript')
GROUP BY year
ORDER BY year;
