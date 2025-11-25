CREATE INDEX IF NOT EXISTS idxregress_mock_items
ON regress.mock_items
    USING bm25 (id, sku, description, (lower(description)::pdb.simple('alias=description_lower')), rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range)
WITH (key_field='id');

-- ensure a cast to `::pdb.regex` works to tokenize using a regular expression
SELECT 'ooh lala'::pdb.regex_pattern('oo|a')::text[];


-- test the `pdb.regex_term` function.  This function was once called `pdb.regex` but it ended up being ambiguous
-- with the tokenizer type `::pdb.regex`, so the function has been renamed to `pdb.regex_term`
SELECT * FROM regress.mock_items WHERE description @@@ pdb.regex('sh.es') ORDER BY id;
SELECT pdb.score(id), * FROM regress.mock_items WHERE description @@@ pdb.regex('sh.es')::pdb.const(42) ORDER BY id;