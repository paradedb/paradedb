-- Total revenue by date
SELECT event_date, SUM(revenue) AS total_revenue
FROM analytics_test
GROUP BY event_date
ORDER BY event_date;

-- Average session duration
SELECT AVG(session_duration) AS average_session_duration
FROM analytics_test;

-- Count of events by event name
SELECT event_name, COUNT(*) AS event_count
FROM analytics_test
GROUP BY event_name
ORDER BY event_count DESC;

-- Maximum page views by user
SELECT user_id, MAX(page_views) AS max_page_views
FROM analytics_test
GROUP BY user_id;

-- Total revenue for a specific event
SELECT SUM(revenue) AS total_revenue
FROM analytics_test
WHERE event_name = 'Purchase';
