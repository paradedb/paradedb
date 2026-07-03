CREATE EXTENSION IF NOT EXISTS vector;
DROP TABLE IF EXISTS cohere_wiki;
CREATE TABLE cohere_wiki (
    _id   text,
    url   text,
    title text,
    text  text,
    emb   vector(1024)
);
