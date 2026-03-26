-- Search users by display name and location
SELECT id, display_name, reputation, location
FROM users
WHERE users @@@ paradedb.parse('location:California')
ORDER BY reputation DESC
LIMIT 100;
