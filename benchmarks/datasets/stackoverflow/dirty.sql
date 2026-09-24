-- Dirty a fraction of heap pages (e.g. 0.5% dirty / 99.5% visible) across tables.
--
-- Postgres tracks visibility per 8KB page in the visibility map. Modifying a single
-- tuple per block (offset 1) clears the all-visible bit for the entire page without
-- generating unnecessary index flush and compactor churn.
--
-- Updating an indexed column disables HOT updates, creating a dead tuple on the
-- sampled page to exercise both visibility map misses and stale ctid handling.

UPDATE stackoverflow_posts
SET score = score
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0
  AND (ctid::text::point)[1]::bigint = 1;

UPDATE badges
SET name = name
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0
  AND (ctid::text::point)[1]::bigint = 1;

UPDATE comments
SET score = score
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0
  AND (ctid::text::point)[1]::bigint = 1;

UPDATE users
SET reputation = reputation
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0
  AND (ctid::text::point)[1]::bigint = 1;
