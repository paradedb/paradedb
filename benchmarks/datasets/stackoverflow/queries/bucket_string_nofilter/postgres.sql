SELECT name, COUNT(*)
FROM badges
GROUP BY name
ORDER BY name;
