create table items (
    id bigserial,
    number bigint
);

create index search_idx on items
using bm25 (id, number) with (key_field='id');

insert into items (id, number) values (1, 12345);
