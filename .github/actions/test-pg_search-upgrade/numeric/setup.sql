create table items (
    id bigserial,
    number numeric
);

create index search_idx on items
using bm25 (id, number) with (key_field='id');

insert into items (id, number) values (1, 0.09809809809809809);
