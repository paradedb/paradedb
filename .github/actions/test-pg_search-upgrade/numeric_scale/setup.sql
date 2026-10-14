-- NOTE: this file runs against the PRIOR extension version, so it must stay valid SQL for the
-- oldest tag in the upgrade matrix (see numeric/setup.sql for why the index says `using bm25`).
--
-- Before #6387, NUMERIC routing bounded only the precision, so a column such as
-- numeric(3, 20) was stored as an `I64` (Numeric64) field. Only NULL and NaN can be indexed at
-- that scale; NULL keeps this fixture independent of how older versions encoded NaN.
create table items (
    id bigserial,
    wide_scale numeric(3, 20)
);

create index search_idx on items
using bm25 (id, wide_scale) with (key_field='id');

insert into items (id, wide_scale) values (1, null);
