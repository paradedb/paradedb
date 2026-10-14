-- NOTE: this file runs against the PRIOR extension version, not the one being built.
-- The `Preserve SQL Files` step copies these fixtures out of the PR head into a
-- tmpdir, then checks out an older tag and installs that version, so everything here
-- must be valid SQL for every tag in the upgrade matrix.

create table items (
    id bigserial,
    number bigint
);

create index search_idx on items
using paradedb (id, number) with (key_field='id');

insert into items (id, number) values (1, 12345);
