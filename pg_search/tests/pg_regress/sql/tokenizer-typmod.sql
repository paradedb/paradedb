SELECT 'Running Shoes.  olé'::pdb.simple::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.simple('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.whitespace::text[];
SELECT 'Running Shoes.  olé'::pdb.whitespace('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.whitespace('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.exact::text[];
SELECT 'Running Shoes.  olé'::pdb.exact('alias=foo')::text[];  -- only option supported for exact
SELECT 'Running Shoes.  olé'::pdb.exact('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.chinese_compatible::text[];
SELECT 'Running Shoes.  olé'::pdb.chinese_compatible('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.chinese_compatible('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.lindera::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=chinese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=japanese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera('language=korean')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.lindera(chinese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(chinese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(japanese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(japanese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(korean, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.lindera(korean, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.jieba::text[];
SELECT 'Running Shoes.  olé'::pdb.jieba('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.jieba('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.ngram::text[]; -- error, needs min/max
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3)::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram('min=2', 'max=3', 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram(2, 3, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::pdb.ngram('min=2', 'max=3', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.stemmed::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::pdb.stemmed(arabic)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(danish)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(dutch)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(english)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(finnish)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(french)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(german)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(greek)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(hungarian)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(italian)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(norwegian)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(portuguese)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(romanian)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(russian)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(spanish)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(swedish)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(tamil)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(turkish)::text[];
SELECT 'Running Shoes.  olé'::pdb.stemmed(foo)::text[]; -- error
SELECT 'Running Shoes.  olé'::pdb.stemmed('language=english', 'lowercase=false', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::pdb.regex::text[]; -- error, needs a regular expression
SELECT 'Running Shoes.  olé'::pdb.regex('ing|oes')::text[];
SELECT 'Running Shoes.  olé'::pdb.regex('ing|oes', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
