CREATE TABLE {{ table }} (
    id BIGINT PRIMARY KEY,
    deleted BOOLEAN,
    type TEXT,
    by TEXT,
    time TIMESTAMPTZ,
    text TEXT,
    dead BOOLEAN,
    parent BIGINT,
    poll BIGINT,
    kids BIGINT[],
    url TEXT,
    score INTEGER,
    title TEXT,
    parts BIGINT[],
    descendants INTEGER,
    emb vector({{ dims }})
);
