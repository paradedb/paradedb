create table items (
    id bigserial,
    numeric64 numeric(15, 9),
    numeric_bytes numeric
);

create index search_idx on items
using bm25 (id, numeric64, numeric_bytes) with (key_field='id');

insert into items (id, numeric64, numeric_bytes) values (1, 0.098098098, 0.09809809809809809);
