\i common/common_setup.sql

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=true')::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.unicode_words('html_strip=true')::text[];
SELECT 'Invalid entities like &nosuchentity; &#xRT; &#-1; s == “&amp hej och hå”`'::pdb.unicode_words('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=false')::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.unicode_words('html_strip=false')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.unicode_words::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=true')::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.unicode_words('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.simple('html_strip=true')::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.simple('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.ngram(5, 5, 'html_strip=true')::text[];
SELECT 'Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.'::pdb.ngram(5, 5, 'html_strip=true')::text[];


DROP TABLE IF EXISTS html_strip_test;
CREATE TABLE html_strip_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO html_strip_test (content) VALUES
('This is a <b>test</b> of the <i>html_strip</i> filter'),
('Tom&apos;s &quot;html_strip&quot; filter handles ampersands like A&amp;B correctly.');

-- with html strip
CREATE INDEX html_strip_test_idx ON html_strip_test USING bm25 (
    id,
    (content::pdb.unicode_words('html_strip=true'))
) WITH (
    key_field = 'id'
);

SELECT pdb.snippet(content, start_tag => '<strong>', end_tag => '</strong>'), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'test' ORDER BY id;
SELECT pdb.snippet(content, start_tag => '<strong>', end_tag => '</strong>'), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'ampersands' ORDER BY id;

-- without html strip
DROP INDEX html_strip_test_idx;
CREATE INDEX html_strip_test_idx ON html_strip_test USING bm25 (
    id,
    content
) WITH (
    key_field = 'id'
);

SELECT pdb.snippet(content, start_tag => '<strong>', end_tag => '</strong>'), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'test' ORDER BY id;
SELECT pdb.snippet(content, start_tag => '<strong>', end_tag => '</strong>'), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'ampersands' ORDER BY id;

DROP TABLE html_strip_test;
