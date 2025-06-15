DROP TABLE IF EXISTS parallel_build_small;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE parallel_build_small (
    id SERIAL PRIMARY KEY,
    name TEXT,
    age INT
);

INSERT INTO parallel_build_small (name, age)
SELECT 'lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.',
20 FROM generate_series(1, 32);