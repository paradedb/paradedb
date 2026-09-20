SELECT name, COUNT(*)
FROM badges
WHERE to_tsvector('english', name) @@ plainto_tsquery('english', 'Question')
GROUP BY name
ORDER BY name;
