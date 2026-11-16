CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_vectorsearch;

DROP TABLE IF EXISTS {{ table }} CASCADE;
CREATE TABLE {{ table }} (
    _id text NOT NULL,
    url text,
    title text,
    text text,
    emb vector({{ dims }}) NOT NULL
);
