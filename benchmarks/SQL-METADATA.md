# Historical SQL metadata

All benchmark runs, values, ranges, units, commits, dates, and non-SQL metadata remain in `data.js`. SQL text from `extra` strings is stored verbatim in `sql-extras.json`, keyed by their SHA-256 hash. The corresponding `extra` value is `sql-extra:sha256:<hash>`. Different query strings retain different hashes. Per-run metadata before `query=` remains inline, and only the SQL suffix is replaced with a reference.

The query charts resolve these references before rendering, preserving SQL and metadata tooltips. Automated baseline comparisons retain numeric values and non-SQL sample metadata. Tools reading SQL metadata directly should resolve the same references.

The benchmark publisher may append ordinary inline extras and reformat `data.js`; the resolver supports both inline and referenced entries. Future compaction must preserve this dictionary and resolve existing references before deduplicating newly appended extras. No retention limit is applied.
