# Docs Reorg Audit

Date: 2026-08-27

Current docs source: `/Users/jamesblackwood-sewell/paradedb/docs`

Proposed docs source: `/Users/jamesblackwood-sewell/paradedb-worktrees/docs/js-docs-reorg/docs`

## Current State

The proposed docs tree now matches the primary tabs at the first directory level:

- `start`
- `guides`
- `reference`
- `concepts`
- `operate`
- `project`

`legacy` remains a separate docs version. `images` and `logo` remain asset directories.

The standalone `choose-your-deployment` and `who-uses-paradedb` pages have been removed. Their content is retained in the current flow:

- Deployment choice now lives in `operate/deploy/overview`.
- Production readiness now lives in `start/go-to-production`.
- Who uses ParadeDB now lives in `start/introduction`.

The Reference tab also collapses the old `query-builder` and `sorting` URL
families into `reference/full-text`, so the lexical search API is easier to
browse from one namespace.

## Coverage Result

Nothing from today's docs is missing from the proposed tree.

| Check                                   | Result |
| --------------------------------------- | -----: |
| Current nav pages                       |    238 |
| Current MDX files                       |    238 |
| Proposed nav pages                      |    302 |
| Proposed MDX files                      |    302 |
| Proposed nav pages missing files        |      0 |
| Proposed MDX files not in proposed nav  |      0 |
| Internal absolute links missing pages   |      0 |
| Internal links to old first-level paths |      0 |
| `mintlify broken-links` failures        |      0 |
| Current public URLs without destination |      0 |
| Redirects generated                     |    238 |

## Redirects

Because the first-level directories now match the tabs, existing public URLs move. The reorg preserves them with `docs/redirects.json`, referenced from `docs/docs.json`.

All 238 current public docs URLs redirect to valid proposed pages.

| Current first-level area | Proposed first-level area | Count |
| ------------------------ | ------------------------- | ----: |
| `welcome`                | `start`                   |     1 |
| `welcome`                | `concepts`                |     3 |
| `welcome`                | `project`                 |     2 |
| `documentation`          | `start`                   |     5 |
| `documentation`          | `reference`               |    72 |
| `documentation`          | `operate`                 |     7 |
| `documentation`          | `concepts`                |     1 |
| `deploy`                 | `operate`                 |    20 |
| `changelog`              | `project`                 |   127 |

## Proposed Additions

The proposed layout adds 64 MDX pages beyond today's docs:

| Area        | Added pages | Notes                                                          |
| ----------- | ----------: | -------------------------------------------------------------- |
| `legacy`    |          39 | Restores older API material under a legacy archive.            |
| `guides`    |           8 | New recipe and decision pages. Several are still placeholders. |
| `concepts`  |           5 | New conceptual split-outs. Several are still placeholders.     |
| `operate`   |           6 | New troubleshooting pages. Several are still placeholders.     |
| `reference` |           3 | New lookup reference pages. Currently placeholders.            |
| `start`     |           3 | Alternatives, first index, and production flow pages.          |

Visible placeholder pages should not ship as final content unless they are replaced with real material.

## Reference Strategy

The concern about Reference mixing API and procedure is valid. The reorg now gives each tab a clearer job:

- Reference answers: "What is the exact syntax, option, operator, function, field type, limitation, or behavior?"
- Guides answer: "Which approach should I choose for this use case?"
- Operate answers: "How do I maintain, verify, tune, recover, or debug this in production?"
- Concepts answer: "How does ParadeDB work, and why is it designed this way?"

Strong fits for Reference:

- `reference/indexing/create-index`: canonical index syntax.
- `reference/indexing/indexing-*`: field and type indexing capabilities.
- `reference/tokenizers/**`: tokenizer behavior and options.
- `reference/token-filters/**`: token filter behavior and options.
- `reference/full-text/match`, `phrase`, `term`, `fuzzy`, `proximity`: query type reference.
- `reference/full-text/query-builder`, `regex`, `range-term`, `phrase-prefix`, `regex-phrase`, `more-like-this`, `query-parser`, and `all`: advanced query function reference.
- `reference/filtering`, `reference/full-text/top-k`, `reference/full-text/score`, `reference/full-text/boost`, and `reference/full-text/highlight`: syntax and behavior references.
- `reference/vector/querying`: vector query syntax.
- `reference/hybrid/**`: hybrid query syntax and RRF behavior.
- `reference/aggregates/**`: aggregate syntax, bucket and metric references, facets, and limitations.
- `reference/joins/overview`: JOIN pushdown behavior reference.
- `reference/operators-and-functions`, `reference/sql-functions`, `reference/configuration`: true lookup references once written.

Moved out of Reference:

- `guides/choosing-a-key-field`: design guidance, not API reference.
- `operate/index-maintenance/reindexing`: operational procedure.
- `operate/index-maintenance/verify-index`: operational procedure and diagnostics.
- `operate/performance-tuning/aggregates`: tuning procedure.

Borderline:

- `reference/vector/tuning` is currently mostly one setting with exact semantics, so it can stay in Reference. If it grows into workload tuning advice, move it to `operate/performance-tuning` and leave a short reference entry behind.

## Content Retention Notes

The Introduction page is intentionally shorter, but the old page's core proof points are retained across the new Start and Concepts flow:

- "Zero ETL Required" and "No second system" are covered by `start/introduction` and `start/alternatives`.
- "Search That Feels Like Postgres" is covered by `start/introduction`.
- "One Index Behind Every Query" is covered by `start/introduction`, `concepts/architecture`, and `concepts/how-the-paradedb-index-works`.
- "As Reliable As Postgres" is covered by `concepts/guarantees`.
- "Production Readiness" is covered by `start/go-to-production`.

The `reference/aggregates/bucket/terms` page had dropped the `JSON Syntax` section from today's docs. That section has been restored so the direct `pdb.agg` terms form is retained.

## Pre-Merge Checks

Keep these checks before merging the reorg:

- Compare current public docs URLs against `docs/redirects.json`.
- Verify every redirect destination exists.
- Verify every proposed nav page has an MDX file.
- Verify every proposed MDX file appears in navigation.
- Verify internal absolute links resolve to real proposed pages.
- Review visible placeholder pages and either fill them or remove them from nav.
