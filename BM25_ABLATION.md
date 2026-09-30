# Pnorms plus MaxScore benchmark branch

This branch starts at ParadeDB `f9ae49951d3f42ed9239ccec7d5018b1fc9cfcae`.
Its Tantivy dependency starts at the baseline's original revision,
`9e5996f669d0ab73bf0511239c4d625661556914`.

The pnorm foundation includes only the integration from ParadeDB `7fd4b8d0f`
and Tantivy's posting-local fieldnorms (`02e5e5eb6`) and direct offsets in
term metadata (`d85adf246`). This is the pnorm implementation present at
ParadeDB `62fd961ac`, without the other changes bundled in that dependency bump.

The SSTable term cache, batched dictionary lookups, conjunction
membership-first optimization, single-pivot WAND optimization, vector changes,
CTID changes, count/MVCC shortcuts, and reader-loading optimizations are excluded.

## Benchmark setup

Build with the same release profile, compiler, PostgreSQL version, features,
and settings as the unmodified `f9ae499` baseline. Create a fresh benchmark
database and index for each variant. Keep the data and segment layout equivalent.
This branch retains the baseline storage layout plus posting norms; it does
not include the later CTID-map component.

Pnorms is opt-in. Add `pnorms=true` to each scored text field's original
tokenizer cast, preserving the tokenizer and other parameters, and rebuild
the index. For example:

```sql
CREATE INDEX documents_search ON documents USING paradedb
    (id, (body::pdb.simple('pnorms=true')));
```

Use the original definition without that option for baseline measurements;
the baseline does not recognize `pnorms`. Existing indexes are not converted
by installing this branch.

This branch adds exactly Tantivy `ccfd11a918e7727082805d23f777feba5f94212c`
(Block MaxScore) and the GUC, query wiring, and tests from ParadeDB
`bdf1132ca8d0a8ddbe260f4003b7b1f9b81bb793` to the companion
`codex/bm25-ablation-pnorms` branch.

The default `auto` mode reproduces that commit's behavior: per segment,
MaxScore handles scored term disjunctions with at least three terms,
at least 256 total postings, and at least one posting per 256 doc IDs.
Other cases keep WAND. To measure forced MaxScore on eligible scored OR
queries, set this in every benchmark connection and label the graph accordingly:

```sql
SET paradedb.disjunction_pruning = 'maxscore';
```

For the original automatic behavior, use:

```sql
SET paradedb.disjunction_pruning = 'auto';
```

Single-term, conjunction, count, and other ineligible query paths do not
become MaxScore queries.

These variants do not include the separate count optimizations from the sprint.
They isolate the effects of pnorms and MaxScore on the selected baseline.

The Git dependency pin supports normal Cargo builds. The Nix Cargo vendor
hash has not been regenerated for these benchmark branches.
