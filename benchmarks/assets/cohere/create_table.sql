CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE {{ table }} (
    _id text,
    url text,
    title text,
    text text,
    emb vector({{ dims }})
);
