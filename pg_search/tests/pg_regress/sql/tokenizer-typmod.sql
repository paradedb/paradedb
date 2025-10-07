SELECT 'Running Shoes.  olé'::paradedb.simple::text[];
SELECT 'Running Shoes.  olé'::paradedb.simple('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.simple('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.whitespace::text[];
SELECT 'Running Shoes.  olé'::paradedb.whitespace('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.whitespace('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.whitespace::text[];
SELECT 'Running Shoes.  olé'::paradedb.whitespace('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.whitespace('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.exact::text[];
SELECT 'Running Shoes.  olé'::paradedb.exact('alias=foo')::text[];  -- only option supported for exact
SELECT 'Running Shoes.  olé'::paradedb.exact('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.chinese_compatible::text[];
SELECT 'Running Shoes.  olé'::paradedb.chinese_compatible('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.chinese_compatible('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];


SELECT 'Running Shoes.  olé'::paradedb.lindera::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::paradedb.lindera('language=chinese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::paradedb.lindera('language=japanese')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::paradedb.lindera('language=korean')::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::paradedb.lindera(chinese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.lindera(chinese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::paradedb.lindera(japanese, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.lindera(japanese, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::paradedb.lindera(korean, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.lindera(korean, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.jieba::text[];
SELECT 'Running Shoes.  olé'::paradedb.jieba('lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.jieba('lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];

SELECT 'Running Shoes.  olé'::paradedb.ngram::text[]; -- error, needs min/max
SELECT 'Running Shoes.  olé'::paradedb.ngram(2, 3)::text[];
SELECT 'Running Shoes.  olé'::paradedb.ngram(2, 3, 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::paradedb.ngram('min=2', 'max=3', 'prefix_only=true')::text[];
SELECT 'Running Shoes.  olé'::paradedb.ngram(2, 3, 'lowercase=false')::text[];
SELECT 'Running Shoes.  olé'::paradedb.ngram(2, 3, 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
SELECT 'Running Shoes.  olé'::paradedb.ngram('min=2', 'max=3', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];


SELECT 'Running Shoes.  olé'::paradedb.stemmed::text[]; -- error, needs a language
SELECT 'Running Shoes.  olé'::paradedb.stemmed(arabic)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(danish)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(dutch)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(english)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(finnish)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(french)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(german)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(greek)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(hungarian)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(italian)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(norwegian)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(portuguese)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(romanian)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(russian)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(spanish)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(swedish)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(tamil)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(turkish)::text[];
SELECT 'Running Shoes.  olé'::paradedb.stemmed(foo)::text[]; -- error
SELECT 'Running Shoes.  olé'::paradedb.stemmed('language=english', 'lowercase=false', 'ascii_folding=true')::text[];


SELECT 'Running Shoes.  olé'::paradedb.regex::text[]; -- error, needs a regular expression
SELECT 'Running Shoes.  olé'::paradedb.regex('ing|oes')::text[];
SELECT 'Running Shoes.  olé'::paradedb.regex('ing|oes', 'lowercase=false', 'stemmer=english', 'ascii_folding=true')::text[];
