<h1 align="center">
  <a href="https://paradedb.com"><img src="docs/logo/readme.svg" alt="ParadeDB"></a>
<br>
</h1>

<p align="center">
  <b>Simple, Elastic-quality search for Postgres</b><br/>
</p>

<h3 align="center">
  <a href="https://docs.paradedb.com">Docs</a> &bull;
  <a href="https://paradedb.com/slack">Slack</a> &bull;
  <a href="https://paradedb.com/blog/">Blog</a> &bull;
  <a href="https://docs.paradedb.com/changelog/">Changelog</a> &bull;
  <a href="https://docs.paradedb.com/welcome/roadmap">Roadmap</a>
</h3>

<p align="center">
  <a href="https://hub.docker.com/r/paradedb/paradedb"><img src="https://img.shields.io/docker/pulls/paradedb/paradedb" alt="Docker Pulls"></a>&nbsp;
  <a href="https://github.com/paradedb/paradedb/stargazers"><img src="https://img.shields.io/github/stars/paradedb/paradedb?style=social&label=Star" alt="GitHub Stars"></a>&nbsp;
  <a href="https://github.com/paradedb/paradedb?tab=AGPL-3.0-1-ov-file#readme"><img src="https://img.shields.io/github/license/paradedb/paradedb?color=blue" alt="License"></a>&nbsp;
  <a href="https://paradedb.com/slack"><img src="https://img.shields.io/badge/Slack-Join%20Community-purple?logo=slack" alt="Slack"></a>&nbsp;
  <a href="https://x.com/paradedb"><img src="https://img.shields.io/twitter/follow/paradedb" alt="Follow @paradedb"></a>
</p>

---

## How to Install

```bash
curl -fsSL https://paradedb.com/install.sh | sh
```

## What is ParadeDB?

[ParadeDB](https://paradedb.com) is a Postgres extension that brings Elastic-quality full-text search, BM25 scoring, fuzzy matching, faceted aggregations, and columnar analytics directly inside Postgres — no ETL pipelines, no external search engines, no data sync headaches.

- [x] **BM25 Full-Text Search** — industry-standard relevance ranking, fuzzy matching, highlighting. [Docs](https://docs.paradedb.com/documentation/full-text/overview)
- [x] **Real-Time Indexing** — data is searchable the instant it's written, with full ACID guarantees. [Docs](https://docs.paradedb.com/welcome/guarantees)
- [x] **Faceted Aggregations** — counts, averages, histograms, and more returned alongside search results. [Docs](https://docs.paradedb.com/documentation/aggregates/facets)
- [x] **Columnar Analytics** — fast filtering, sorting, and aggregates via built-in columnar storage. [Docs](https://docs.paradedb.com/documentation/indexing/columnar)
- [x] **Hybrid Search** — combine BM25 text search with pgvector similarity search. [Docs](https://docs.paradedb.com/documentation/full-text/overview)
- [x] **JOINs** — search across normalized tables without denormalization. [Docs](https://docs.paradedb.com/documentation/joins/overview)
- [x] **Standard SQL** — no custom DSL, works with every Postgres tool, ORM, and driver

## How It Works

ParadeDB is a Postgres extension — it runs inside your existing database, not alongside it.

1. **Install** `pg_search` into any Postgres 15+ instance, or use the [ParadeDB Docker image](https://hub.docker.com/r/paradedb/paradedb) which comes pre-configured
2. **Create a BM25 index** on any table. The index is a covering index that stores all indexed columns in both an inverted index (for full-text search) and a columnar index (for fast analytics)
3. **Query with SQL**. ParadeDB introduces custom operators like `|||` for search. When a query uses these operators, ParadeDB's custom scan takes over — pushing filters, aggregates, and sorting directly into the index for maximum performance

Under the hood, the BM25 index is built on an [LSM tree](https://docs.paradedb.com/welcome/architecture) powered by [Tantivy](https://github.com/quickwit-oss/tantivy) (a Rust-based search library inspired by Lucene). Writes are buffered in memory and flushed as immutable segments, making inserts and updates fast. Reads are automatically parallelized across Postgres workers.

```sql
-- Create an index
CREATE INDEX idx ON docs USING bm25 (id, title, body, rating) WITH (key_field='id');

-- Search with BM25 scoring
SELECT title, pdb.score(id) FROM docs WHERE body ||| 'search ranking' ORDER BY score DESC LIMIT 10;

-- Fuzzy search
SELECT title FROM docs WHERE title ||| 'postgras~1';

-- Faceted aggregation alongside results
SELECT title, pdb.agg('{"terms": {"field": "rating"}}') OVER () FROM docs WHERE body ||| 'search';
```

For full documentation, visit [docs.paradedb.com](https://docs.paradedb.com).

## Why ParadeDB?

|                           | **Postgres (tsvector)** | **Elasticsearch** | **ParadeDB** |
| ------------------------- | :---------------------: | :---------------: | :----------: |
| BM25 relevance scoring    |         &cross;         |      &check;      |   &check;    |
| Fuzzy matching            |         &cross;         |      &check;      |   &check;    |
| Faceted aggregations      |         &cross;         |      &check;      |   &check;    |
| Columnar analytics        |         &cross;         |      &check;      |   &check;    |
| Highlighting & snippets   |         Partial         |      &check;      |   &check;    |
| Real-time indexing        |         &check;         |      &cross;      |   &check;    |
| ACID transactions         |         &check;         |      &cross;      |   &check;    |
| SQL interface             |         &check;         |      &cross;      |   &check;    |
| JOINs                     |         &check;         |      &cross;      |   &check;    |
| No ETL / data sync        |         &check;         |      &cross;      |   &check;    |
| Built for updates/deletes |         &check;         |      &cross;      |   &check;    |

## Features

### Full-Text Search

BM25 relevance ranking, phrase matching, fuzzy search with typo tolerance, regex, proximity queries, and more-like-this — all through simple SQL operators.

```sql
-- Fuzzy search with typo tolerance
SELECT title FROM docs WHERE title ||| 'postgras~1';

-- Phrase matching
SELECT title FROM docs WHERE title ||| '"full-text search"';

-- Highlighting
SELECT title, pdb.snippet(body) FROM docs WHERE body ||| 'search';
```

### Faceted Aggregations

Return search results and aggregate analytics in a single query — counts, averages, histograms, and more.

```sql
SELECT title, pdb.agg('{"terms": {"field": "rating"}}') OVER ()
FROM docs
WHERE body ||| 'search'
ORDER BY pdb.score(id) DESC
LIMIT 5;
```

### Columnar Analytics

Non-text fields are automatically stored in columnar format, enabling fast filtering, sorting, and aggregation pushdown directly into the index.

### Hybrid Search

Combine BM25 full-text search with pgvector similarity search for semantic + keyword hybrid queries.

### Top K Optimization

Highly optimized `ORDER BY ... LIMIT` queries with automatic parallelization across workers for sub-millisecond response times on large datasets.

### JOINs

Search across normalized Postgres tables with `INNER`, `LEFT`, `RIGHT`, `FULL`, `CROSS`, and `LATERAL` joins — no denormalization required.

## Integrations & Ecosystem

ParadeDB works with any Postgres-compatible tool. The Docker image comes with these extensions pre-installed:

| Extension                                        | Purpose                        |
| ------------------------------------------------ | ------------------------------ |
| [pgvector](https://github.com/pgvector/pgvector) | Vector similarity search       |
| [PostGIS](https://postgis.net/)                  | Geospatial queries             |
| [pg_cron](https://github.com/citusdata/pg_cron)  | Background jobs and scheduling |
| [pg_ivm](https://github.com/sraoss/pg_ivm)       | Incremental materialized views |

**ORMs & Frameworks:** Django, Rails, Prisma, and any ORM that speaks SQL.

**Infrastructure:** AWS RDS (via [logical replication](https://docs.paradedb.com/deploy/logical-replication/getting-started)), GCP Cloud SQL, Azure, Kubernetes, Docker.

## Deployment

<table>
  <tr>
    <td><b>Docker</b></td>
    <td><code>docker run paradedb/paradedb</code></td>
    <td><a href="https://docs.paradedb.com/deploy/self-hosted/docker">Guide</a></td>
  </tr>
  <tr>
    <td><b>Kubernetes</b></td>
    <td>Helm chart via CloudNativePG</td>
    <td><a href="https://docs.paradedb.com/deploy/self-hosted/kubernetes">Guide</a></td>
  </tr>
  <tr>
    <td><b>Extension</b></td>
    <td>Install <code>pg_search</code> into existing Postgres 15+</td>
    <td><a href="https://docs.paradedb.com/deploy/self-hosted/extension">Guide</a></td>
  </tr>
  <tr>
    <td><b>Logical Replication</b></td>
    <td>Replicate from RDS, Aurora, Cloud SQL, AlloyDB</td>
    <td><a href="https://docs.paradedb.com/deploy/logical-replication/getting-started">Guide</a></td>
  </tr>
</table>

Prebuilt binaries available for Debian, Ubuntu, RHEL, and macOS on the [Releases](https://github.com/paradedb/paradedb/releases) page.

## Trusted in Production

ParadeDB launched in the **Y Combinator S23** batch and has been deployed **over 400,000 times** in the past 12 months. It powers search and analytics at:

<table>
  <tr>
    <td width="25%" align="center"><b>Alibaba Cloud</b><br/>Asia-Pacific's largest cloud provider<br/><a href="https://www.paradedb.com/customers/case-study-alibaba">Case study</a></td>
    <td width="25%" align="center"><b>Bilt Rewards</b><br/>$36B+ in processed payments<br/><a href="https://www.paradedb.com/customers/case-study-bilt">Case study</a></td>
    <td width="25%" align="center"><b>Modern Treasury</b><br/>Automated payment operations</td>
    <td width="25%" align="center"><b>Span</b><br/>AI developer productivity</td>
  </tr>
</table>

## Community & Support

- [Slack](https://paradedb.com/slack) — ask questions, share what you're building
- [GitHub Discussions](https://github.com/paradedb/paradedb/discussions) — longer-form Q&A
- [GitHub Issues](https://github.com/paradedb/paradedb/issues/new/choose) — bug reports and feature requests
- [Email](mailto:sales@paradedb.com) — enterprise support and commercial licensing

## Contributing

We welcome contributions of all sizes! Check out our [good first issues](https://github.com/paradedb/paradedb/labels/good%20first%20issue) to get started, or join [Slack](https://paradedb.com/slack) to connect with the team. See our [Contributing Guide](/CONTRIBUTING.md) for details.

This project is released with a [Contributor Code of Conduct](/CODE_OF_CONDUCT.md). By participating, you agree to follow its terms.

## License

ParadeDB is licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software. For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).
