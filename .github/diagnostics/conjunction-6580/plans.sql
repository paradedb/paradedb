\pset tuples_only on
\pset format unaligned
SET statement_timeout = '120s';
\echo INDEX_METADATA
SELECT json_build_object('filenode', pg_relation_filenode('documents_body_bm25_idx'), 'bytes', pg_relation_size('documents_body_bm25_idx'), 'segments', (SELECT json_agg(s) FROM pdb.index_segments('documents_body_bm25_idx') s), 'index_info', (SELECT json_agg(s) FROM paradedb.index_info('documents_body_bm25_idx') s));
\echo TABLE_STATISTICS
SELECT row_to_json(s) FROM pg_stats s WHERE tablename='documents';
\echo SETTINGS
SELECT json_object_agg(name, setting) FROM pg_settings;
PREPARE topq(text) AS SELECT id, body, pdb.score(id) AS score FROM documents WHERE body @@@ pdb.parse($1, lenient => true) ORDER BY score DESC LIMIT 10;
SET plan_cache_mode = force_generic_plan;
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('then AND you AND traverse AND it AND with AND iterators');
\o
\echo PLAN force_generic_plan 162:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('then AND you AND traverse AND it AND with AND iterators');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('new AND object AND to AND represent AND an AND order AND if AND none');
\o
\echo PLAN force_generic_plan 214:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('new AND object AND to AND represent AND an AND order AND if AND none');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND conduit AND and AND then AND use AND that AND to AND pull AND the AND wire AND through');
\o
\echo PLAN force_generic_plan 166:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND conduit AND and AND then AND use AND that AND to AND pull AND the AND wire AND through');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('column AND a AND and AND b AND are AND editable AND now');
\o
\echo PLAN force_generic_plan 466:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('column AND a AND and AND b AND are AND editable AND now');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('code AND pre AND p AND that AND should AND be');
\o
\echo PLAN force_generic_plan 267:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('code AND pre AND p AND that AND should AND be');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND inverse AND as AND the AND inverse AND are AND going AND to AND depend AND on');
\o
\echo PLAN force_generic_plan 762:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND inverse AND as AND the AND inverse AND are AND going AND to AND depend AND on');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('detected AND on AND the AND trajectory AND by');
\o
\echo PLAN force_generic_plan 898:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('detected AND on AND the AND trajectory AND by');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND example AND of AND what AND you AND mean AND by AND array AND of AND pointers AND would AND it AND just AND be');
\o
\echo PLAN force_generic_plan 993:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND example AND of AND what AND you AND mean AND by AND array AND of AND pointers AND would AND it AND just AND be');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('null AND the AND process AND is AND no AND different AND for AND any AND other AND character AND that AND you AND want');
\o
\echo PLAN force_generic_plan 1107:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('null AND the AND process AND is AND no AND different AND for AND any AND other AND character AND that AND you AND want');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND you AND need AND to AND bind AND code AND this AND code AND when AND you AND created AND a AND function');
\o
\echo PLAN force_generic_plan 1232:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND you AND need AND to AND bind AND code AND this AND code AND when AND you AND created AND a AND function');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('group AND by AND parent AND a AND code AND pre AND p AND current AND output AND parent');
\o
\echo PLAN force_generic_plan 339:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('group AND by AND parent AND a AND code AND pre AND p AND current AND output AND parent');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('d.firstchild.nodevalue AND code AND pre AND p AND you AND should AND use AND the AND same AND method AND to AND access AND the');
\o
\echo PLAN force_generic_plan 1172:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('d.firstchild.nodevalue AND code AND pre AND p AND you AND should AND use AND the AND same AND method AND to AND access AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('following AND code AND p AND pre AND code AND include AND lt AND stdio.h AND gt AND include AND lt AND wchar.h AND gt AND include');
\o
\echo PLAN force_generic_plan 237:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('following AND code AND p AND pre AND code AND include AND lt AND stdio.h AND gt AND include AND lt AND wchar.h AND gt AND include');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('how AND are AND you AND generating AND this AND dataframe AND in AND the');
\o
\echo PLAN force_generic_plan 1012:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('how AND are AND you AND generating AND this AND dataframe AND in AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND links AND when AND one AND of AND the AND links AND is AND clicked AND i AND would');
\o
\echo PLAN force_generic_plan 1245:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND links AND when AND one AND of AND the AND links AND is AND clicked AND i AND would');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('is AND what AND you AND need AND or AND not AND p AND p AND by AND the AND way AND open AND office');
\o
\echo PLAN force_generic_plan 225:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('is AND what AND you AND need AND or AND not AND p AND p AND by AND the AND way AND open AND office');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i AND am AND getting AND error AND the AND localization AND variable AND loc.installfilesactiontext AND is AND unknown AND please AND ensure AND the AND variable AND is');
\o
\echo PLAN force_generic_plan 212:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i AND am AND getting AND error AND the AND localization AND variable AND loc.installfilesactiontext AND is AND unknown AND please AND ensure AND the AND variable AND is');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND a AND package AND in AND customization AND code AND one AND appends AND something AND to AND a AND list AND with AND p');
\o
\echo PLAN force_generic_plan 310:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND a AND package AND in AND customization AND code AND one AND appends AND something AND to AND a AND list AND with AND p');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('other AND options AND in AND the AND end AND of AND the AND world AND sense AND are AND _apocalypse_ AND or');
\o
\echo PLAN force_generic_plan 322:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('other AND options AND in AND the AND end AND of AND the AND world AND sense AND are AND _apocalypse_ AND or');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('get AND round AND the AND painting AND painted AND issue AND by AND saying AND is AND a AND picture AND painted AND by AND in AND 1896');
\o
\echo PLAN force_generic_plan 1083:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('get AND round AND the AND painting AND painted AND issue AND by AND saying AND is AND a AND picture AND painted AND by AND in AND 1896');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND this AND imho AND nullifies AND the AND value AND of AND so AND and AND shows AND the AND real AND attitude');
\o
\echo PLAN force_generic_plan 1098:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND this AND imho AND nullifies AND the AND value AND of AND so AND and AND shows AND the AND real AND attitude');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND dsu AND and AND the AND changes AND made AND in AND 2.4 AND http AND tinyurl.com AND 99gfpu AND google AND books');
\o
\echo PLAN force_generic_plan 278:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND dsu AND and AND the AND changes AND made AND in AND 2.4 AND http AND tinyurl.com AND 99gfpu AND google AND books');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('certainly AND sounds AND a AND like AND colossus AND the AND forbin AND project AND and AND the');
\o
\echo PLAN force_generic_plan 499:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('certainly AND sounds AND a AND like AND colossus AND the AND forbin AND project AND and AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('book AND of AND gelfand AND and AND manin AND when AND i AND searched AND for AND that AND so AND im AND confused AND now AND as');
\o
\echo PLAN force_generic_plan 708:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('book AND of AND gelfand AND and AND manin AND when AND i AND searched AND for AND that AND so AND im AND confused AND now AND as');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('experience AND as AND marnes AND i AND wonder AND if AND it AND has AND to AND do AND with AND unpacked AND extensions');
\o
\echo PLAN force_generic_plan 640:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('experience AND as AND marnes AND i AND wonder AND if AND it AND has AND to AND do AND with AND unpacked AND extensions');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('that AND column AND could AND i AND plug AND in AND a AND metallic AND hydrogen AND rocket AND p');
\o
\echo PLAN force_generic_plan 124:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('that AND column AND could AND i AND plug AND in AND a AND metallic AND hydrogen AND rocket AND p');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('can AND you AND kindly AND add AND the AND adjustments AND or AND final AND code AND in AND answers AND so');
\o
\echo PLAN force_generic_plan 211:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('can AND you AND kindly AND add AND the AND adjustments AND or AND final AND code AND in AND answers AND so');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('going AND to AND keep AND both');
\o
\echo PLAN force_generic_plan 426:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('going AND to AND keep AND both');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('brain AND is AND not AND cooperating');
\o
\echo PLAN force_generic_plan 574:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('brain AND is AND not AND cooperating');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('looks AND valid AND to');
\o
\echo PLAN force_generic_plan 1198:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('looks AND valid AND to');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND if AND statement AND there AND is AND a AND difference AND when AND you');
\o
\echo PLAN force_generic_plan 929:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND if AND statement AND there AND is AND a AND difference AND when AND you');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('buddy AND we AND came AND across AND your AND stuff AND a AND couple AND of AND weeks AND ago AND love');
\o
\echo PLAN force_generic_plan 1117:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('buddy AND we AND came AND across AND your AND stuff AND a AND couple AND of AND weeks AND ago AND love');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('select AND product AND from AND product AND where');
\o
\echo PLAN force_generic_plan 1059:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('select AND product AND from AND product AND where');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('on AND properties AND that');
\o
\echo PLAN force_generic_plan 360:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('on AND properties AND that');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i\''d AND name AND it AND userid AND improving AND your AND style AND here');
\o
\echo PLAN force_generic_plan 1090:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i\''d AND name AND it AND userid AND improving AND your AND style AND here');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('this AND addresses AND the AND question AND asked AND you AND can AND find AND more AND information AND on AND how AND to AND write');
\o
\echo PLAN force_generic_plan 33:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('this AND addresses AND the AND question AND asked AND you AND can AND find AND more AND information AND on AND how AND to AND write');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('which AND leads AND to AND the AND function AND including AND it AND in AND the AND return AND value AND which AND i AND want AND to');
\o
\echo PLAN force_generic_plan 1240:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('which AND leads AND to AND the AND function AND including AND it AND in AND the AND return AND value AND which AND i AND want AND to');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('codeigniter AND model AND what AND is AND the AND best AND way');
\o
\echo PLAN force_generic_plan 645:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('codeigniter AND model AND what AND is AND the AND best AND way');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('you AND search AND as AND a AND term AND i\''m AND not AND sure AND where AND you AND are AND getting AND at');
\o
\echo PLAN force_generic_plan 22:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('you AND search AND as AND a AND term AND i\''m AND not AND sure AND where AND you AND are AND getting AND at');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('it AND fails AND that AND dump AND file AND is AND empty');
\o
\echo PLAN force_generic_plan 873:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('it AND fails AND that AND dump AND file AND is AND empty');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND pre AND code AND request AND processing AND failed AND nested AND exception AND is');
\o
\echo PLAN force_generic_plan 1095:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND pre AND code AND request AND processing AND failed AND nested AND exception AND is');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('figured AND it AND out AND you AND can AND download AND the AND source AND from AND this');
\o
\echo PLAN force_generic_plan 874:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('figured AND it AND out AND you AND can AND download AND the AND source AND from AND this');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('and AND wine AND is AND a AND lot AND more AND popular AND and AND more');
\o
\echo PLAN force_generic_plan 1190:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('and AND wine AND is AND a AND lot AND more AND popular AND and AND more');
SET plan_cache_mode = force_custom_plan;
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('then AND you AND traverse AND it AND with AND iterators');
\o
\echo PLAN force_custom_plan 162:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('then AND you AND traverse AND it AND with AND iterators');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('new AND object AND to AND represent AND an AND order AND if AND none');
\o
\echo PLAN force_custom_plan 214:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('new AND object AND to AND represent AND an AND order AND if AND none');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND conduit AND and AND then AND use AND that AND to AND pull AND the AND wire AND through');
\o
\echo PLAN force_custom_plan 166:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND conduit AND and AND then AND use AND that AND to AND pull AND the AND wire AND through');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('column AND a AND and AND b AND are AND editable AND now');
\o
\echo PLAN force_custom_plan 466:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('column AND a AND and AND b AND are AND editable AND now');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('code AND pre AND p AND that AND should AND be');
\o
\echo PLAN force_custom_plan 267:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('code AND pre AND p AND that AND should AND be');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND inverse AND as AND the AND inverse AND are AND going AND to AND depend AND on');
\o
\echo PLAN force_custom_plan 762:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('the AND inverse AND as AND the AND inverse AND are AND going AND to AND depend AND on');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('detected AND on AND the AND trajectory AND by');
\o
\echo PLAN force_custom_plan 898:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('detected AND on AND the AND trajectory AND by');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND example AND of AND what AND you AND mean AND by AND array AND of AND pointers AND would AND it AND just AND be');
\o
\echo PLAN force_custom_plan 993:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND example AND of AND what AND you AND mean AND by AND array AND of AND pointers AND would AND it AND just AND be');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('null AND the AND process AND is AND no AND different AND for AND any AND other AND character AND that AND you AND want');
\o
\echo PLAN force_custom_plan 1107:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('null AND the AND process AND is AND no AND different AND for AND any AND other AND character AND that AND you AND want');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND you AND need AND to AND bind AND code AND this AND code AND when AND you AND created AND a AND function');
\o
\echo PLAN force_custom_plan 1232:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND you AND need AND to AND bind AND code AND this AND code AND when AND you AND created AND a AND function');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('group AND by AND parent AND a AND code AND pre AND p AND current AND output AND parent');
\o
\echo PLAN force_custom_plan 339:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('group AND by AND parent AND a AND code AND pre AND p AND current AND output AND parent');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('d.firstchild.nodevalue AND code AND pre AND p AND you AND should AND use AND the AND same AND method AND to AND access AND the');
\o
\echo PLAN force_custom_plan 1172:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('d.firstchild.nodevalue AND code AND pre AND p AND you AND should AND use AND the AND same AND method AND to AND access AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('following AND code AND p AND pre AND code AND include AND lt AND stdio.h AND gt AND include AND lt AND wchar.h AND gt AND include');
\o
\echo PLAN force_custom_plan 237:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('following AND code AND p AND pre AND code AND include AND lt AND stdio.h AND gt AND include AND lt AND wchar.h AND gt AND include');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('how AND are AND you AND generating AND this AND dataframe AND in AND the');
\o
\echo PLAN force_custom_plan 1012:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('how AND are AND you AND generating AND this AND dataframe AND in AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND links AND when AND one AND of AND the AND links AND is AND clicked AND i AND would');
\o
\echo PLAN force_custom_plan 1245:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND links AND when AND one AND of AND the AND links AND is AND clicked AND i AND would');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('is AND what AND you AND need AND or AND not AND p AND p AND by AND the AND way AND open AND office');
\o
\echo PLAN force_custom_plan 225:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('is AND what AND you AND need AND or AND not AND p AND p AND by AND the AND way AND open AND office');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i AND am AND getting AND error AND the AND localization AND variable AND loc.installfilesactiontext AND is AND unknown AND please AND ensure AND the AND variable AND is');
\o
\echo PLAN force_custom_plan 212:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i AND am AND getting AND error AND the AND localization AND variable AND loc.installfilesactiontext AND is AND unknown AND please AND ensure AND the AND variable AND is');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND a AND package AND in AND customization AND code AND one AND appends AND something AND to AND a AND list AND with AND p');
\o
\echo PLAN force_custom_plan 310:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND a AND package AND in AND customization AND code AND one AND appends AND something AND to AND a AND list AND with AND p');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('other AND options AND in AND the AND end AND of AND the AND world AND sense AND are AND _apocalypse_ AND or');
\o
\echo PLAN force_custom_plan 322:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('other AND options AND in AND the AND end AND of AND the AND world AND sense AND are AND _apocalypse_ AND or');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('get AND round AND the AND painting AND painted AND issue AND by AND saying AND is AND a AND picture AND painted AND by AND in AND 1896');
\o
\echo PLAN force_custom_plan 1083:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('get AND round AND the AND painting AND painted AND issue AND by AND saying AND is AND a AND picture AND painted AND by AND in AND 1896');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND this AND imho AND nullifies AND the AND value AND of AND so AND and AND shows AND the AND real AND attitude');
\o
\echo PLAN force_custom_plan 1098:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND this AND imho AND nullifies AND the AND value AND of AND so AND and AND shows AND the AND real AND attitude');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND dsu AND and AND the AND changes AND made AND in AND 2.4 AND http AND tinyurl.com AND 99gfpu AND google AND books');
\o
\echo PLAN force_custom_plan 278:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('of AND dsu AND and AND the AND changes AND made AND in AND 2.4 AND http AND tinyurl.com AND 99gfpu AND google AND books');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('certainly AND sounds AND a AND like AND colossus AND the AND forbin AND project AND and AND the');
\o
\echo PLAN force_custom_plan 499:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('certainly AND sounds AND a AND like AND colossus AND the AND forbin AND project AND and AND the');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('book AND of AND gelfand AND and AND manin AND when AND i AND searched AND for AND that AND so AND im AND confused AND now AND as');
\o
\echo PLAN force_custom_plan 708:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('book AND of AND gelfand AND and AND manin AND when AND i AND searched AND for AND that AND so AND im AND confused AND now AND as');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('experience AND as AND marnes AND i AND wonder AND if AND it AND has AND to AND do AND with AND unpacked AND extensions');
\o
\echo PLAN force_custom_plan 640:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('experience AND as AND marnes AND i AND wonder AND if AND it AND has AND to AND do AND with AND unpacked AND extensions');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('that AND column AND could AND i AND plug AND in AND a AND metallic AND hydrogen AND rocket AND p');
\o
\echo PLAN force_custom_plan 124:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('that AND column AND could AND i AND plug AND in AND a AND metallic AND hydrogen AND rocket AND p');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('can AND you AND kindly AND add AND the AND adjustments AND or AND final AND code AND in AND answers AND so');
\o
\echo PLAN force_custom_plan 211:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('can AND you AND kindly AND add AND the AND adjustments AND or AND final AND code AND in AND answers AND so');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('going AND to AND keep AND both');
\o
\echo PLAN force_custom_plan 426:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('going AND to AND keep AND both');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('brain AND is AND not AND cooperating');
\o
\echo PLAN force_custom_plan 574:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('brain AND is AND not AND cooperating');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('looks AND valid AND to');
\o
\echo PLAN force_custom_plan 1198:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('looks AND valid AND to');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND if AND statement AND there AND is AND a AND difference AND when AND you');
\o
\echo PLAN force_custom_plan 929:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('an AND if AND statement AND there AND is AND a AND difference AND when AND you');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('buddy AND we AND came AND across AND your AND stuff AND a AND couple AND of AND weeks AND ago AND love');
\o
\echo PLAN force_custom_plan 1117:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('buddy AND we AND came AND across AND your AND stuff AND a AND couple AND of AND weeks AND ago AND love');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('select AND product AND from AND product AND where');
\o
\echo PLAN force_custom_plan 1059:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('select AND product AND from AND product AND where');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('on AND properties AND that');
\o
\echo PLAN force_custom_plan 360:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('on AND properties AND that');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i\''d AND name AND it AND userid AND improving AND your AND style AND here');
\o
\echo PLAN force_custom_plan 1090:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('i\''d AND name AND it AND userid AND improving AND your AND style AND here');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('this AND addresses AND the AND question AND asked AND you AND can AND find AND more AND information AND on AND how AND to AND write');
\o
\echo PLAN force_custom_plan 33:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('this AND addresses AND the AND question AND asked AND you AND can AND find AND more AND information AND on AND how AND to AND write');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('which AND leads AND to AND the AND function AND including AND it AND in AND the AND return AND value AND which AND i AND want AND to');
\o
\echo PLAN force_custom_plan 1240:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('which AND leads AND to AND the AND function AND including AND it AND in AND the AND return AND value AND which AND i AND want AND to');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('codeigniter AND model AND what AND is AND the AND best AND way');
\o
\echo PLAN force_custom_plan 645:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('codeigniter AND model AND what AND is AND the AND best AND way');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('you AND search AND as AND a AND term AND i\''m AND not AND sure AND where AND you AND are AND getting AND at');
\o
\echo PLAN force_custom_plan 22:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('you AND search AND as AND a AND term AND i\''m AND not AND sure AND where AND you AND are AND getting AND at');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('it AND fails AND that AND dump AND file AND is AND empty');
\o
\echo PLAN force_custom_plan 873:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('it AND fails AND that AND dump AND file AND is AND empty');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND pre AND code AND request AND processing AND failed AND nested AND exception AND is');
\o
\echo PLAN force_custom_plan 1095:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('p AND pre AND code AND request AND processing AND failed AND nested AND exception AND is');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('figured AND it AND out AND you AND can AND download AND the AND source AND from AND this');
\o
\echo PLAN force_custom_plan 874:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('figured AND it AND out AND you AND can AND download AND the AND source AND from AND this');
\o /dev/null
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('and AND wine AND is AND a AND lot AND more AND popular AND and AND more');
\o
\echo PLAN force_custom_plan 1190:conjunction
EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) EXECUTE topq('and AND wine AND is AND a AND lot AND more AND popular AND and AND more');
DEALLOCATE topq;
