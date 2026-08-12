-- pg_search 0.25.0.3 -> 0.25.0.4 (dev): intentionally empty.
--
-- This step originally created the measure-less pdb.top(n) fusion-arm
-- annotation type, whose C symbols no longer exist: 0.25.0.5 replaced it
-- with the measure-named pdb.top_bm25(n)/pdb.top_knn(n). Databases that
-- were at 0.25.0.4 when those symbols existed are handled by the 0.25.0.4
-- -> 0.25.0.5 script (its DROP ... IF EXISTS tolerates both states), so
-- upgrade chains passing through here create nothing.
SELECT 1;
