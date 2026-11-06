# Compare clusters built by the extensions

`compare.py` loads one set of vectors into PostgreSQL, runs real `CREATE INDEX`
commands for pg_search and pg_vectorsearch, and exports their persisted centroids
and memberships. It never replaces memberships with nearest-centroid assignments.

Use a local PostgreSQL instance with both patched extensions installed, Python,
NumPy, and psycopg 3. The input NPZ must contain float matrices named `vectors`
and `queries` with the same dimension. For large datasets, use a directory with
`vectors.npy` and `queries.npy`; these are memory-mapped, normalized in batches,
and never loaded as one full matrix. The runner creates a new schema and leaves
its table and indexes available for inspection.

```sh
python compare.py --url 'postgresql:///cluster_exports' \
  --dataset /path/to/dataset.npz --output /path/to/new-results-directory
```

Both indexes use their native defaults, with cosine distance and no `WITH`
options. Sampling, cluster count, segment count, refinement, quantization, and
replication remain each extension's own choices. The runner verifies that extension
GUCs equal their boot defaults and that both index catalogs have no reloptions.
PostgreSQL memory and worker settings are inherited from the same server and
recorded in the manifest, alongside resolved PRISM settings and pg_search segment
metadata. Native training limits remain in effect; training budgets are not matched.

```sql
CREATE INDEX pdb_clusters ON items USING paradedb (id, emb vector_cosine_ops);
CREATE INDEX prism_clusters ON items USING prism (emb vec32_cosine_ops);
```

```sh
python compare.py --url 'postgresql:///cluster_exports' \
  --dataset /path/to/msmarco-v1-vectors --dataset-name 'MS MARCO v1' \
  --output /path/to/new-results-directory
```

All dataset rows are used unless `--rows` explicitly limits the prefix. The
manifest records the source row count and any explicit limit. Exact top-10 ground
truth is computed in batches, and memberships stream through a server cursor.
Only the optional global nearest-centroid diagnostic samples rows (16,384 by
default, fixed seed 0); set `--nearest-sample-rows 0` to check every row. Assigned
cosine loss, cluster sizes, and recall/work curves always cover the full corpus.
The extension SQL functions themselves can still materialize their output on the
server, so the database host needs memory and temporary disk for that export.

The manual Cohere workflow runs `run_ci.sh` on the existing benchmark runner. It
restores the published Cohere 10m heap snapshot from
`s3://paradedb-benchmark/snapshots/cohere/v1/10m/pg_backrest`, verifies exactly 10,000,000 rows,
exports all embeddings with binary COPY, and reads the published 100-query fixture.
`prepare_cohere.py` can also export an existing local heap. The cloud run uses the
common benchmark PostgreSQL settings (12 GiB maintenance memory and shared buffers,
eight parallel workers), while leaving both extensions' index settings at defaults.
It saves artifacts without publishing a baseline or posting GitHub comments.

The output contains raw centroid arrays and parallel `row_ids` / `labels` arrays
with one entry per stored posting, including replicas. Every table row must have
at least one membership, every referenced centroid must exist, and exported
membership counts must match posting-list sizes. Multiple pg_search segments are
supported. A row in multiple PRISM clusters contributes once to recall, at the
first visited cluster containing it; every scanned replica still counts as posting
work. Selected recall points are checked by exact reranking of unique actual
cluster members. Flat segments have no IVF memberships; the runner rejects an
incomplete export rather than silently excluding their rows.

Per-row cosine loss uses the best of that row's actual stored memberships.
Posting-weighted cosine loss is also reported, along with replication factor and
the fraction of rows replicated. The nearest-centroid diagnostic does not change
any assignment.

The recall/work curves use exact ordering of exported leaf centroids and count
posting rows scanned (which can exceed the corpus size with replication). They
measure clustering and membership quality, including
the effect of each extension's build-time assignment. They do not measure the
production query router, vector quantization during query scoring, or SQL query
latency. Run multiple independent builds before drawing conclusions about small
quality differences.

## Export APIs

```sql
SELECT * FROM paradedb.vector_cluster_centroids('my_pdb_index', 'emb');
-- index_oid, segno, cluster_id, centroid real[]
SELECT * FROM paradedb.vector_cluster_members('my_pdb_index', 'emb');
-- index_oid, segno, cluster_id, heap_tid

SELECT * FROM prism_cluster_centroids('my_prism_index');
-- cluster_id, centroid real[], centroid_kind
SELECT * FROM prism_cluster_members('my_prism_index');
-- heap_tid, cluster_id
```

The functions require ownership and read the built index without changing it.
pg_search cluster IDs are scoped by partition index and full segment UUID; flat
segments have no clusters and emit no rows. PRISM exports primary and replica
memberships when replication is enabled, skipping entries already marked dead.
It preserves the existing behavior of `prism_tids_clusters(index, tids)`.

TIDs identify physical tuples. Export from a quiescent table, hold a repeatable-read
snapshot and a table SHARE lock, and join each exported TID to the table's `ctid`
to obtain stable IDs. The join excludes rows not visible to the snapshot even if
VACUUM has not removed their index entries. For partitioned tables, use the exported
index OID to identify the correct heap partition; TIDs alone are not globally unique.

PRISM's exported centroid is the posting head's full-precision **encoding
reference**, transformed back from its stored rotation. It is labeled
`encoding_reference`. With leaf refinement disabled on a fresh index, it is the
trained leaf centroid up to floating-point rotation error. After refinement or
maintenance it may differ from the routing centroid; compressed routing pages
cannot recover the original centroid exactly. No index format or clustering
algorithm changes are needed for these diagnostics.

These patches target fresh benchmark installations. An existing installation needs
the new function declarations as well as the new extension library.
