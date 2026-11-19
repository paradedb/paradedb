CREATE TABLE {{ table }} (
    _id   text NOT NULL,
    url   text,
    title text,
    text  text,
    emb   vector({{ dims }}) NOT NULL
);
