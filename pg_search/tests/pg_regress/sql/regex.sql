-- ensure a cast to `::pdb.regex` works to tokenize using a regular expression
SELECT 'ooh lala'::pdb.regex('oo|a')::text[];


-- test the `pdb.regex_term` function.  This function was once called `pdb.regex` but it ended up being ambiguous
-- with the tokenizer type `::pdb.regex`, so the function has been renamed to `pdb.regex_term`
SELECT * FROM regress.mock_items WHERE description @@@ pdb.regex_term('sh.es') ORDER BY id;
SELECT paradedb.score(id), * FROM regress.mock_items WHERE description @@@ pdb.regex_term('sh.es')::pdb.const(42) ORDER BY id;