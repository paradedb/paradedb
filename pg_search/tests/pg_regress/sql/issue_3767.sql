CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS testcore CASCADE;

SET enable_indexscan TO off;

create table testcore
(
    id       serial not null
constraint pkey_core_id primary key,
    dwf_doid bigint,
    author varchar[]
);

INSERT INTO testcore (dwf_doid, author)
SELECT
    row_number() OVER () AS dwf_doid,
    author
FROM (
    SELECT '{Brian Griffin}'::varchar[]         AS author, 1 AS ct
    UNION ALL SELECT '{Tricia Takanawa}',         670
    UNION ALL SELECT '{Stewie Griffin}',          618
    UNION ALL SELECT '{God}',                     622
    UNION ALL SELECT '{Horace}',                  703
    UNION ALL SELECT '{Tom Tucker}',              653
    UNION ALL SELECT '{Mayor Adam West}',         586
    UNION ALL SELECT '{Bonnie Swanson}',          633
    UNION ALL SELECT '{Diane Simmons}',           663
    UNION ALL SELECT '{Joe Swanson}',             683
    UNION ALL SELECT '{Fouad}',                   674
    UNION ALL SELECT '{Evil Monkey}',             628
    UNION ALL SELECT '{Chris Griffin}',           666
    UNION ALL SELECT '{Joyce Kinney}',            579
    UNION ALL SELECT '{James Woods}',             621
    UNION ALL SELECT '{Principal Shephard}',      622
    UNION ALL SELECT '{Karen Griffin}',           680
    UNION ALL SELECT '{Meg Griffin}',             657
    UNION ALL SELECT '{Carl}',                    613
    UNION ALL SELECT '{Mort Goldman}',            679
    UNION ALL SELECT '{Glenn Quagmire}',          675
    UNION ALL SELECT '{Barabara Pewterschmidt}',  654
    UNION ALL SELECT '{Mickey McFinnigan}',       627
    UNION ALL SELECT '{Brian Griffin}',           1
    UNION ALL SELECT '{Peter Griffin}',           618
    UNION ALL SELECT '{Consuela}',                670
    UNION ALL SELECT '{Thelma Griffin}',          642
    UNION ALL SELECT '{Lois Griffin}',            617
    UNION ALL SELECT '{Cleveland Brown}',         637
    UNION ALL SELECT '{Carter Pewterschmidt}',    634
    UNION ALL SELECT '{Ollie Williams}',          617
) t,
LATERAL generate_series(1, t.ct);

CREATE INDEX textidx_parade_core ON testcore
USING bm25 (dwf_doid, author)
WITH (key_field='dwf_doid');

-- Running this repeatedly with pauses is the best way to repro the issue
-- At some point, Postgres decides that all the results are visible/don't need to be heap checked
-- which is what leads us down the code path where null values get returned in the target list
SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, '1' FROM testcore WHERE author @@@ pdb.term('brian');
SELECT pg_sleep(0.5);

SELECT dwf_doid, 2 + 2 FROM testcore WHERE author @@@ pdb.term('brian');
SELECT dwf_doid, NULL FROM testcore WHERE author @@@ pdb.term('brian');

DROP TABLE testcore;
