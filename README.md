<h1 align="center">
  <a href="https://paradedb.com"><img src="docs/logo/readme.svg" alt="ParadeDB"></a>
<br>
</h1>

<p align="center">
  <b>Simple, Elastic-quality search for Postgres</b><br/>
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Docs</a> &bull;
  <a href="https://paradedb.com/slack">Community</a> &bull;
  <a href="https://paradedb.com/blog/">Blog</a> &bull;
  <a href="https://docs.paradedb.com/changelog/">Changelog</a>
</h3>

<p align="center">
  <a href="https://hub.docker.com/r/paradedb/paradedb"><img src="https://img.shields.io/docker/pulls/paradedb/paradedb" alt="Docker Pulls"></a>&nbsp;
  <a href="https://github.com/paradedb/paradedb?tab=AGPL-3.0-1-ov-file#readme"><img src="https://img.shields.io/github/license/paradedb/paradedb?color=blue" alt="License"></a>&nbsp;
  <a href="https://paradedb.com/slack"><img src="https://img.shields.io/badge/Community-Join%20Slack-purple?logo=slack" alt="Community"></a>&nbsp;
  <a href="https://x.com/paradedb"><img src="https://img.shields.io/twitter/follow/paradedb" alt="Follow @paradedb"></a>
</p>

---

## How to Install

```bash
curl -fsSL https://paradedb.com/install.sh | sh
```

When you're ready to deploy, check out our [hosting options](https://docs.paradedb.com/deploy/overview).

## What is ParadeDB?

[ParadeDB](https://paradedb.com) is a Postgres extension that brings Elastic-quality full-text search, BM25 scoring, fuzzy matching, faceted aggregations, and columnar analytics directly inside Postgres — no ETL pipelines, no external search engines, no data sync headaches.

- [x] **BM25 Full-Text Search** — industry-standard relevance ranking, fuzzy matching, highlighting. [Docs](https://docs.paradedb.com/documentation/full-text/overview)
- [x] **Top K Optimization** — highly optimized `ORDER BY ... LIMIT` with automatic parallelization. [Docs](https://docs.paradedb.com/documentation/sorting/topk)
- [x] **Real-Time Indexing** — data is searchable the instant it's written, with full ACID guarantees. [Docs](https://docs.paradedb.com/welcome/guarantees)
- [x] **Faceted Aggregations** — counts, averages, histograms, and more returned alongside search results. [Docs](https://docs.paradedb.com/documentation/aggregates/facets)
- [x] **Columnar Analytics** — fast filtering, sorting, and aggregates via built-in columnar storage. [Docs](https://docs.paradedb.com/documentation/indexing/columnar)
- [x] **JOINs** — search across normalized tables without denormalization. [Docs](https://docs.paradedb.com/documentation/joins/overview)
- [x] **Standard SQL** — no custom DSL, works with every Postgres tool, ORM, and driver
- [x] **Hybrid Search** — combine BM25 text search with pgvector similarity search
- [ ] **Vector Search** — performant filtered vector search without pgvector (coming soon)

Star and watch this repository to follow along. See our [current projects](https://github.com/paradedb/paradedb/projects?query=is%3Aopen) and [long-term roadmap](https://docs.paradedb.com/welcome/roadmap).

## How It Works

ParadeDB assembles best-in-class open-source components into a single Postgres extension. We use [pgrx](https://github.com/pgcentralfoundation/pgrx) to bridge Postgres and Rust, [Tantivy](https://github.com/quickwit-oss/tantivy) for full-text search, and [Apache DataFusion](https://github.com/apache/datafusion) for OLAP processing. The result is Elastic-quality search and analytics that is 100% Postgres native — install it directly into your existing Postgres or run it as a [sidecar replica cluster](https://cloudnative-pg.io/docs/1.28/replica_cluster/) with native Postgres replication.

It introduces a new index type, the BM25 index, which stores indexed columns in both an inverted index (for search) and a columnar index (for analytics). Query it with standard SQL — ParadeDB's custom scan pushes filters, aggregates, and sorting directly into the index. For a deeper dive, see our [CMU Database Group talk](https://db.cs.cmu.edu/events/building-blocks-paradedb-philippe-noel/) or [architecture docs](https://docs.paradedb.com/welcome/architecture).

```sql
-- Create an index
CREATE INDEX idx ON docs USING bm25 (id, title, body, rating) WITH (key_field='id');

-- Search with BM25 scoring
SELECT title, pdb.score(id) AS score FROM docs WHERE body ||| 'search ranking' ORDER BY score DESC LIMIT 10;

-- Fuzzy search with typo tolerance
SELECT title FROM docs WHERE title ||| 'postgras'::pdb.fuzzy(1);

-- Phrase matching
SELECT title FROM docs WHERE title ### '"full-text search"';

-- Highlighting
SELECT title, pdb.snippet(body) FROM docs WHERE body ||| 'search';

-- Faceted aggregation alongside results
SELECT title, pdb.agg('{"terms": {"field": "rating"}}') OVER ()
FROM docs
WHERE body ||| 'search'
ORDER BY pdb.score(id) DESC
LIMIT 5;
```

For full documentation, visit [docs.paradedb.com](https://docs.paradedb.com).

## Integrations

ParadeDB integrates with the tools you already use, with more coming.

### ORMs & Frameworks

- [Django](https://github.com/paradedb/django-paradedb) — native Django ORM integration for ParadeDB queries
- [SQLAlchemy](https://github.com/paradedb/sqlalchemy-paradedb) — Python SQLAlchemy support
- [Rails](https://github.com/paradedb/rails-paradedb) — Active Record integration for Ruby on Rails
- More coming (Prisma, and others)

### AI Agents

- [Agent Skills](https://github.com/paradedb/agent-skills) — give AI coding agents full ParadeDB knowledge via `npx skills add paradedb/agent-skills`
- [MCP Integration](https://docs.paradedb.com/mcp) — Model Context Protocol support for AI assistants

### PaaS & Cloud Platforms

- [Render](https://docs.paradedb.com/deploy/cloud-platforms/render) — one-click deploy
- [Railway](https://docs.paradedb.com/deploy/cloud-platforms/railway) — one-click deploy
- [DigitalOcean](https://docs.paradedb.com/deploy/cloud-platforms/digitalocean) — droplet deployment
- More coming (Heroku, and others)

## Community & Support

- [Slack](https://paradedb.com/slack) — ask questions, share what you're building
- [GitHub Discussions](https://github.com/paradedb/paradedb/discussions) — longer-form Q&A
- [GitHub Issues](https://github.com/paradedb/paradedb/issues/new/choose) — bug reports and feature requests
- [Email](mailto:sales@paradedb.com) — enterprise support and commercial licensing

## Contributing

We welcome contributions of all sizes! Check out our [good first issues](https://github.com/paradedb/paradedb/labels/good%20first%20issue) to get started. For larger contributions, we recommend discussing them with us in [Slack](https://paradedb.com/slack) first. See our [Contributing Guide](/CONTRIBUTING.md) and [Code of Conduct](/CODE_OF_CONDUCT.md) for details.

## License

ParadeDB Community is licensed under the [GNU Affero General Public License v3.0](LICENSE). For [ParadeDB Enterprise](https://docs.paradedb.com/deploy/enterprise) licensing, contact [sales@paradedb.com](mailto:sales@paradedb.com).
