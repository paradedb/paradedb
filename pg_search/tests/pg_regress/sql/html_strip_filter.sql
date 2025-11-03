\i common/common_setup.sql

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=true')::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.unicode_words('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=false')::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.unicode_words('html_strip=false')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.unicode_words::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.unicode_words('html_strip=true')::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.unicode_words('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.simple('html_strip=true')::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.simple('html_strip=true')::text[];

SELECT 'The <b>test</b> of the <i>html_strip</i> filter'::pdb.ngram(5, 5, 'html_strip=true')::text[];
SELECT 'The &lt;b&gt;test&lt;/b&gt; of the &lt;i&gt;html_strip&lt;/i&gt; filter'::pdb.ngram(5, 5, 'html_strip=true')::text[];


DROP TABLE IF EXISTS html_strip_test;
CREATE TABLE html_strip_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO html_strip_test (content) VALUES
('This is a <b>test</b> of the <i>html_strip</i> filter'),
('There&apos;s a test of the <code>html_strip</code> filter');

CREATE INDEX ON html_strip_test USING bm25 (
    id,
    (content::pdb.unicode_words('html_strip=true'))
) WITH (
    key_field = 'id'
);

SELECT pdb.snippet(content), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'test' ORDER BY id;
SELECT pdb.snippet(content), pdb.snippet_positions(content) FROM html_strip_test WHERE content === 'the' ORDER BY id;
