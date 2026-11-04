CREATE EXTENSION IF NOT EXISTS vector;
DROP TABLE IF EXISTS msmarco;
CREATE TABLE msmarco (
    _id   text NOT NULL,
    url   text,
    title text,
    text  text,
    emb   vector(1024) NOT NULL
);
