-- Dirty {{ dirty_pct }}% of blocks across stackoverflow tables by updating indexed columns.
UPDATE stackoverflow_posts
SET score = score
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0;

UPDATE badges
SET name = name
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0;

UPDATE comments
SET score = score
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0;

UPDATE users
SET reputation = reputation
WHERE (ctid::text::point)[0]::bigint % {{ dirty_modulus }} = 0;
