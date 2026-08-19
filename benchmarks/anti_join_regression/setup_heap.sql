\set ON_ERROR_STOP on

DROP TABLE IF EXISTS anti_bench_owned;
DROP TABLE IF EXISTS anti_bench_library;

CREATE TABLE anti_bench_library (
    id bigint PRIMARY KEY,
    title text NOT NULL,
    category text NOT NULL
);

CREATE TABLE anti_bench_owned (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id bigint NOT NULL,
    item_id bigint NOT NULL
);

CREATE INDEX anti_bench_owned_user_item_idx
    ON anti_bench_owned (user_id, item_id);
