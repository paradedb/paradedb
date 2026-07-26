create table legacy_inet_items (
    id integer,
    ip inet,
    ips inet[]
);

create index legacy_inet_idx on legacy_inet_items
using bm25 (id, ip, ips) with (key_field = 'id');

insert into legacy_inet_items values
    (1, '10.0.0.1', ARRAY['192.168.0.1', '192.168.0.2']::inet[]),
    (2, '10.0.0.2', ARRAY['192.168.0.3', '192.168.0.4']::inet[]);
