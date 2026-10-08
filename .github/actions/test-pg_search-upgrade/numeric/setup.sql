-- NOTE: this file runs against the PRIOR extension version, not the one being built.
-- The `Preserve SQL Files` step copies these fixtures out of the PR head into a
-- tmpdir, then checks out an older tag and installs that version, so everything here
-- must be valid SQL for every tag in the upgrade matrix.

create table items (
    id bigserial,
    numeric64 numeric(15, 9),
    numeric_bytes numeric
);

create index search_idx on items
using paradedb (id, numeric64, numeric_bytes) with (key_field='id');

insert into items (id, numeric64, numeric_bytes) values (1, 0.098098098, 0.09809809809809809);
