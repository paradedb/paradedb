DROP TABLE IF EXISTS parallel_build_large;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE parallel_build_large (
    id SERIAL PRIMARY KEY,
    name TEXT
);

INSERT INTO parallel_build_large (name)
SELECT 'lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.'
FROM generate_series(1, 35000);
