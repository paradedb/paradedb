window.BENCHMARK_DATA = {
  "lastUpdate": 1770565570146,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'docs' (25M rows)": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "22434f12d7eec5084167804afd22b34f86810f09",
          "message": "fix: Allow the custom scan to be used in parallel plans without its own workers (#4109)\n\n## What\n\n* Split `set_parallel_safe` from `set_parallel` for `CustomScan`s, and\nalways mark the `basescan` as `parallel_safe`.\n* Disable #4077 for joins.\n\n## Why\n\nAfter #4077, two things happened to the first benchmark query in\n`benchmarks/datasets/docs/queries/pg_search/hierarchical_content-scores-large.sql`\n(and likely others):\n\n### Loss of parallel safety\n\nThe query (which was previously using `Normal` custom scans) was failing\nto get the custom scan at all, and was instead falling back to the IAM\n(which cannot produce scores):\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=804922.46..804924.96 rows=1000 width=3048)\n   ->  Sort  (cost=804922.46..804940.37 rows=7161 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n         ->  Hash Join  (cost=594228.68..804529.83 rows=7161 width=3048)\n               Hash Cond: (files.\"documentId\" = documents.id)\n-> Gather (cost=571487.12..688316.81 rows=144249 width=2070)\n                     Workers Planned: 7\n-> Parallel Hash Join (cost=570487.12..672891.91 rows=20607 width=2070)\n                           Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                                 Table: pages\n                                 Index: pages_index\n                                 Segment Count: 8\n                                 Exec Method: NormalScanExecState\n                                 Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=566064.09..566064.09 rows=31202 width=1030)\n-> Parallel Index Scan using files_index on files (cost=10.00..566064.09\nrows=31202 width=1030)\nIndex Cond: (id @@@\n'{\"with_index\":{\"oid\":2096822,\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}'::paradedb.searchqueryinput)\n               ->  Hash  (cost=1561.36..1561.36 rows=155136 width=986)\n-> Custom Scan (ParadeDB Scan) on documents (cost=10.00..1561.36\nrows=155136 width=986)\n                           Table: documents\n                           Index: documents_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n(27 rows)\n</details>\n\nThe reason for this is that #4077 caused us to determine that because\nthe scan was scanning fewer than 300k rows, it probably didn't need\nparallel workers.\n\nBut `set_parallel` was _also_ the only place where we were claiming that\nour custom scan is `parallel_safe`. And a plan must be parallel safe to\nbe used inside of any _other_ parallel scan.\n\n### No participation in parallel hash joins\n\nAfter fixing the above, we got the custom scan, but the plan was subtly\ndifferent from before:\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=188822.03..188822.06 rows=10 width=3048)\n   ->  Sort  (cost=188822.03..188839.93 rows=7161 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n         ->  Gather  (cost=87220.00..188667.28 rows=7161 width=3048)\n               Workers Planned: 7\n-> Hash Join (cost=86220.00..186951.18 rows=1023 width=3048)\n                     Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                           Table: pages\n                           Index: pages_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Hash (cost=84184.19..84184.19 rows=7745 width=2016)\n-> Hash Join (cost=22751.54..84184.19 rows=7745 width=2016)\nHash Cond: (files.\"documentId\" = documents.id)\n-> Custom Scan (ParadeDB Scan) on files (cost=10.00..1570.12 rows=156012\nwidth=1030)\n                                       Table: files\n                                       Index: files_index\n                                       Segment Count: 8\n                                       Exec Method: NormalScanExecState\n                                       Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Hash (cost=1561.35..1561.35 rows=155135 width=986)\n-> Custom Scan (ParadeDB Scan) on documents (cost=10.00..1561.35\nrows=155135 width=986)\n                                             Table: documents\n                                             Index: documents_index\n                                             Segment Count: 8\nExec Method: NormalScanExecState\n                                             Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n</details>\n\nRather than being able to participate in a parallel hash join with\nparallel independent sorts, the two smaller tables were instead being\nscanned sequentially into a Gather, and _then_ sorted.\n\nThis lead to a total cost of 188k, which was sufficient on CI machines\nto trigger JIT compilation, and cause queries long enough to cause\ntimeouts.\n\nDisabling #4077 in the context of joins allowed the two smaller tables\nto participate in the plan.\n\n## How\n\n* Added `set_parallel_safe`, and used it universally in the `basescan`,\nand added an additional branch to `init_search_reader` to handle the\ncase when we are part of a parallel plan, but without our own parallel\nstate.\n* Disabled #4077 in the presence of joins, and clarified the\nrelationship with the `uses_correlated_vars` flag.\n* Made a quick driveby fix to ensure that our estimates match the actual\nnumber of emitted tuples.\n\nThe final restored plan looks like:\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=16558.60..16559.83 rows=10 width=3048)\n   ->  Gather Merge  (cost=16558.60..17428.92 rows=7084 width=3048)\n         Workers Planned: 7\n         ->  Sort  (cost=15558.48..15561.01 rows=1012 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n-> Parallel Hash Join (cost=10564.17..15536.61 rows=1012 width=3048)\n                     Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                           Table: pages\n                           Index: pages_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=10540.35..10540.35 rows=1106 width=2016)\n-> Parallel Hash Join (cost=2861.14..10540.35 rows=1106 width=2016)\nHash Cond: (files.\"documentId\" = documents.id)\n-> Parallel Custom Scan (ParadeDB Scan) on files (cost=10.00..205.02\nrows=19502 width=1030)\n                                       Table: files\n                                       Index: files_index\n                                       Segment Count: 8\n                                       Exec Method: NormalScanExecState\n                                       Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=203.84..203.84 rows=19384 width=986)\n-> Parallel Custom Scan (ParadeDB Scan) on documents (cost=10.00..203.84\nrows=19384 width=986)\n                                             Table: documents\n                                             Index: documents_index\n                                             Segment Count: 8\nExec Method: NormalScanExecState\n                                             Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n</details>\n\n## Tests\n\nBenchmark queries are able to run with both a parallel plan and the\ncustom scan again.\n\nThis was really difficult to reproduce outside of the benchmark harness:\nit requires a large enough dataset to trigger a parallel plan on a\nparent node. I spent at least an hour trying to repro it in a regress\ntest, but failed.",
          "timestamp": "2026-02-06T09:04:47-08:00",
          "tree_id": "767b23c3a564e81c3f4ba39e2e4ac753fefa9bc0",
          "url": "https://github.com/paradedb/paradedb/commit/22434f12d7eec5084167804afd22b34f86810f09"
        },
        "date": 1770400935214,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13801.432499999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13603.584,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 545.5405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 539.3199999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3266.5775,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3325.9375,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 151.8055,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1194.274,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1176.0615,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1173.777,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.0535,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 656.6514999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1458.5255,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 725.9704999999999,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2631.4615000000003,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 693.7535,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2614.434,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 20.601,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 43.718,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 52.932,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 699.649,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1869.1635,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 592.6959999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1635.2245,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a8323d7d2ed7a46ed4c398882be595628edca559",
          "message": "chore: Add missing AGPL license headers to Rust source files (#4124)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\n47 Rust source files were missing the standard AGPL-3.0 license header\ncomment. This adds the header to all of them so every `.rs` file in the\nrepo is consistent.\n\n## Why\n\nAll source files should carry the AGPL license header for legal\ncompliance and consistency. These files were added over time without it.\n\n## How\n\n- Identified all `.rs` files (excluding `target/`) missing the `//\nCopyright (c) 2023-2026 ParadeDB, Inc.` header\n- Prepended the standard 16-line AGPL header to each file, matching the\nexact format used across the rest of the codebase\n- Files span `benchmarks/`, `macros/`, `pg_search/`, `stressgres/`,\n`tests/`, and `tokenizers/`\n\n## Tests\n\nNo functional changes — header comments only. `cargo check`, `fmt`, and\n`clippy` all pass via pre-commit hooks.\n\nCo-authored-by: Claude Opus 4.6 <noreply@anthropic.com>",
          "timestamp": "2026-02-07T10:45:17-05:00",
          "tree_id": "627b799a5aaeb8f0076d7bcda8b95173dee601ae",
          "url": "https://github.com/paradedb/paradedb/commit/a8323d7d2ed7a46ed4c398882be595628edca559"
        },
        "date": 1770482606282,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13768.731,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13781.057499999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 563.6145,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 560.2945,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3417.415,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3416.0355,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 143.833,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1205.846,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1167.1595,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1162.5865,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.287,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 654.5419999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1449.63,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 726.44,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2636.0485,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 693.6355,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2623.9635,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 19.963,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 42.513000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 52.6265,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 712.0625,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1871.625,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 586.043,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1648.8815,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "92ea39f880584773012a49399594be22062837c9",
          "message": "chore: Prune unused code in customscan module (#4113)\n\n## What\n\nRemove a module-level `#![allow(unused_variables)]`, and clean up the\ndead code that was exposed.\n\n## Why\n\nLess dead code.",
          "timestamp": "2026-02-07T17:00:52-08:00",
          "tree_id": "a91615eefbaf3d289f8dd1a28dba8088a7bac479",
          "url": "https://github.com/paradedb/paradedb/commit/92ea39f880584773012a49399594be22062837c9"
        },
        "date": 1770516010949,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13894.3135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13875.626,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 560.5005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 576.3205,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3530.6245,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3511.1455,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 143.749,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1197.479,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1166.7669999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1172.3635,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 654.4855,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 654.372,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1441.1505,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 726.6355000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2633.9660000000003,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 693.3544999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2635.464,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 19.441000000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 39.691,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 50.034499999999994,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 697.4445000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1853.9,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 585.8695,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1638.4405,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "86703ba13832ce310a25f4dc74dedee8350515e2",
          "message": "perf: Always claim `ORDER BY` path keys in the join scan (#4120)\n\n## What\n\nAlways claim all `ORDER BY` path keys in the join scan, since it is\nrequired to take over all sorting in order to be able to apply the\n`LIMIT`.\n\n## Why\n\nTo simplify plans by removing unnecessary sorts.",
          "timestamp": "2026-02-07T17:45:59-08:00",
          "tree_id": "2741fdee77a011a43729d4e0e7f72bcea62eabb8",
          "url": "https://github.com/paradedb/paradedb/commit/86703ba13832ce310a25f4dc74dedee8350515e2"
        },
        "date": 1770518617289,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13843.382,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13799.139500000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 564.8354999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 567.357,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3448.6245,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3513.779,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 145.37,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1189.193,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1179.644,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1170.9285,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 648.115,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 647.452,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1466.44,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 714.426,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2553.4325,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 687.508,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2547.5175,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 19.9005,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 42.146,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 53.0205,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 724.14,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1850.261,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 594.9045,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1621.721,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a4db272a4d1f7d9f7dc17fd2f4b32213a12f4bff",
          "message": "feat: improve `PgSearchScan` display (in Joins) in `EXPLAIN` output (#4121)\n\n## Ticket(s) Closed\n\n- Partially helps #4061\n\n## What\n\nDisplay the actual Tantivy query in `PgSearchScan` nodes within EXPLAIN\noutput, instead of just showing `PgSearchScan` with no details.\n\n## Why\n\nPreviously, EXPLAIN output for JoinScan queries showed `PgSearchScan`\nwith no indication of what query was being executed against the index.\nThis made it difficult to understand and debug query plans, especially\nwhen troubleshooting filter pushdown behavior.\n\n## How\n\n- Added `query_for_display: Option<SearchQueryInput>` field to\n`SegmentPlan`\n- Updated `DisplayAs` implementation to show the query using\n`explain_format()`:\n  - `PgSearchScan: \"all\"` when no filter is applied\n- `PgSearchScan: {...}` showing the JSON query when a filter is pushed\ndown\n- Pass the query through to `SegmentPlan::new` from\n`TableProvider::scan` and `MixedFastFieldExecState`\n\n## Tests\n\n- Updated `join_custom_scan` regression test expected output to reflect\nthe new display format",
          "timestamp": "2026-02-07T23:53:14-08:00",
          "tree_id": "679a1f535a166323d3e81559832883d46bce3092",
          "url": "https://github.com/paradedb/paradedb/commit/a4db272a4d1f7d9f7dc17fd2f4b32213a12f4bff"
        },
        "date": 1770540689343,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13758.0805,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13749.342499999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 568.6405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 567.9245000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3493.1594999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3456.2425000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 146.1835,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1192.4235,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1181.5845,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1174.9375,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 652.7094999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 652.5315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1452.086,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 724.8969999999999,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2616.1220000000003,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 690.217,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2600.8495000000003,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 19.457500000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 42.9075,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 53.527,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 704.1895,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1861.2734999999998,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 587.555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1621.6464999999998,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "140686560+dask-58@users.noreply.github.com",
            "name": "Dhruv",
            "username": "dask-58"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f249a4d10045aad10d15daab760bd2debbdc4f17",
          "message": "fix: handle varchar/text casts in tokenizer and phrase functions (#4104)\n\n# Ticket(s) Closed\n\n- Closes #3667\n\n## What\nAdd implicit `text`/`varchar` casts to tokenizer types and field names,\nand add a regression test covering `varchar`/`text` inputs for\n`pdb.tokenize_unicode_words` and `paradedb.phrase`.\n\n## Why\nORMs commonly cast string literals to `varchar` or `text`. Previously,\nthis caused `\"function does not exist\"` errors due to strict function\nand operator resolution in Postgres for v2 functions and tokenizer\ntypes.\n\n## How\n- Generate implicit `text`/`varchar` casts for tokenizer input types\n(skipping `pdb.alias` to avoid duplicate cast definitions).\n- Add an implicit `varchar → paradedb.fieldname` cast.\n- Add a pg_regress test: `varchar_text_casts`.\n\n## Tests\n- `cargo pgrx regress pg17 varchar_text_casts`\n- Manually verified with:\n  ```sql\n  SELECT pdb.tokenize_unicode_words('good job'::varchar);\n  SELECT pdb.tokenize_unicode_words('good job'::text);\n  ```\n\n---------\n\nSigned-off-by: dask-58 <140686560+dask-58@users.noreply.github.com>\nSigned-off-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nSigned-off-by: dask-58 <googldhruv@gmail.com>\nSigned-off-by: Dhruv <140686560+dask-58@users.noreply.github.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2026-02-08T08:23:54-05:00",
          "tree_id": "55ee1ac0189ac1954817631d739fd6de7d3def31",
          "url": "https://github.com/paradedb/paradedb/commit/f249a4d10045aad10d15daab760bd2debbdc4f17"
        },
        "date": 1770560501125,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13796.2195,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13720.743,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 541.9465,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 558.8465,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3413.558,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3363.9935,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 144.89499999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1191.1865,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1164.701,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1160.5895,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 658.5715,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 658.861,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1453.797,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 725.5395000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2626.2405,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 697.788,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2640.044,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 20.176000000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 42.837,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 52.532,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 711.162,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1838.0995,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 594.7375,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1639.15,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2d0bb19fa3554b0f7b1ee52f4bf9d1cb6520a8fc",
          "message": "chore: Fix typos and grammar across docs and source (#4133)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nFixed 6 typos and grammar issues found across docs and source:\n\n- `docs/documentation/full-text/highlight.mdx`: \"the the\" → \"the\"\n(duplicate word)\n- `docs/legacy/full-text/highlighting.mdx`: \"the the\" → \"the\" (duplicate\nword)\n- `pg_search/src/index/reader/index.rs`: \"to to\" → \"to\" (duplicate word)\n- `pg_search/src/api/config.rs`: \"sine\" → \"since\" in user-facing\ndeprecation warning\n- `pg_search/src/postgres/scan.rs`: \"varrying\" → \"varying\"\n- `pg_search/src/schema/mod.rs`: double negative (\"Neither JSON nor\nrange fields are not yet sortable\") → \"JSON and range fields are not yet\nsortable\"\n\n## Why\n\nThe `config.rs` one is user-facing. The rest are just cleanup.\n\n## How\n\n^\n\n## Tests\n\nNo functional changes — text only.\n\nCo-authored-by: Claude Opus 4.6 <noreply@anthropic.com>",
          "timestamp": "2026-02-08T09:48:34-05:00",
          "tree_id": "997b557bfb129ebf8179253ce1b07fd67fd507f4",
          "url": "https://github.com/paradedb/paradedb/commit/2d0bb19fa3554b0f7b1ee52f4bf9d1cb6520a8fc"
        },
        "date": 1770565565304,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_sort",
            "value": 13823.733499999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 13878.608,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, MAX(p.\"createdAt\") as last_activity FROM files f JOIN pages p ON f.id = p.\"fileId\" WHERE f.content @@@ 'Section' GROUP BY f.id, f.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search",
            "value": 573.048,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "disjunctive_search - alternative 1",
            "value": 567.9915000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT f.id, f.title, paradedb.score(f.id) as score FROM files f LEFT JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PARENT_GROUP_2%' AND ( f.title @@@ 'Title' OR d.title @@@ 'Title' ) ORDER BY score DESC LIMIT 10"
          },
          {
            "name": "distinct_parent_sort",
            "value": 3509.4555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3479.9535,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT d.id, d.title, d.parents FROM documents d JOIN files f ON d.id = f.\"documentId\" JOIN pages p ON f.id = p.\"fileId\" WHERE p.\"sizeInBytes\" > 5000 AND d.parents LIKE 'SFR%' ORDER BY d.title ASC LIMIT 50"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 135.8885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 1201.335,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\", d.title as document_title FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE d.parents LIKE 'PROJECT_ALPHA%' AND f.title @@@ 'collab12' ORDER BY f.\"createdAt\" DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1161.084,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 1155.5285,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.1795,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 654.815,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1439.123,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 728.204,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 2636.8435,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 693.787,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 2625.875,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "paging-string-max",
            "value": 19.188499999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 42.709,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 52.293,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 703.123,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "permissioned_search - alternative 1",
            "value": 1875.6774999999998,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, paradedb.score(f.id) as relevance FROM files f JOIN documents d ON f.\"documentId\" = d.id WHERE f.title @@@ 'File' AND d.parents LIKE 'PARENT_GROUP_10%' ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "semi_join_filter",
            "value": 573.7825,
            "unit": "median ms",
            "extra": "SET paradedb.enable_join_custom_scan TO off; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 1653.7485,
            "unit": "median ms",
            "extra": "SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT f.id, f.title, f.\"createdAt\" FROM files f WHERE  f.\"documentId\" IN ( SELECT id FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1' ) ORDER BY f.title ASC LIMIT 25"
          }
        ]
      }
    ]
  }
}