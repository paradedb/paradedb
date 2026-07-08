<h1 align="center">
  <a href="https://paradedb.com">
    <picture align=center>
      <source media="(prefers-color-scheme: dark)" srcset="docs/logo/paradedb-logo-dark-large.svg">
      <source media="(prefers-color-scheme: light)" srcset="docs/logo/paradedb-logo-light-large.svg">
      <img alt="The ParadeDB logo." src="docs/logo/paradedb-logo-light-large.svg">
    </picture>
  </a>
  <br>
</h1>

<p align="center">
  <b>Search without a second system.</b><br/>
  One Postgres for your application data, full-text search, vector retrieval, and aggregations.
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

## Installation

To install ParadeDB locally in a fresh Docker container and drop straight into a `psql` session:

```bash
curl -fsSL https://paradedb.com/install.sh | sh
```

When you're ready to deploy, check out our [hosting options](https://docs.paradedb.com/deploy/overview).

## What is ParadeDB?

[ParadeDB](https://paradedb.com) adds Elastic-quality full-text search, vector retrieval, and aggregations to Postgres with the `pg_search` extension. Your application data and your search engine live in one database, with no second system to deploy and nothing to sync.

Vectors are currently indexed using the [pgvector](https://github.com/pgvector/pgvector) extension, but native vector support is coming to our search index soon.

- [x] [Full-Text Search](https://docs.paradedb.com/documentation/full-text/overview)
  - [x] [BM25 Scoring](https://docs.paradedb.com/documentation/sorting/score)
  - [x] [Top K](https://docs.paradedb.com/documentation/sorting/topk)
  - [x] [Highlighting](https://docs.paradedb.com/documentation/full-text/highlight)
  - [x] [Tokenizers & Token Filters](https://docs.paradedb.com/documentation/tokenizers/overview)
- [x] [Filtering](https://docs.paradedb.com/documentation/filtering)
- [x] [Aggregates](https://docs.paradedb.com/documentation/aggregates/overview)
  - [x] [Columnar Storage](https://docs.paradedb.com/documentation/indexing/columnar)
  - [x] [Bucket & Metrics](https://docs.paradedb.com/documentation/aggregates/overview)
  - [x] [Facets](https://docs.paradedb.com/documentation/aggregates/facets)
- [x] [JOINs](https://docs.paradedb.com/documentation/joins/overview)
- [ ] Native Vector Search (coming soon)
- [ ] Native Hybrid Search (coming soon)

Star and watch this repository to follow along. See our [current projects](https://github.com/paradedb/paradedb/projects?query=is%3Aopen) and [long-term roadmap](https://docs.paradedb.com/welcome/roadmap).

## How It Works

ParadeDB integrates battle-tested Rust libraries for search and analytics inside Postgres, contributing upstream whenever possible. Our primary dependencies are:

- [pgrx](https://github.com/pgcentralfoundation/pgrx) — bridges Postgres and Rust
- [Tantivy](https://github.com/quickwit-oss/tantivy) — powers full-text search
- [Apache DataFusion](https://github.com/apache/datafusion) — handles OLAP processing

For a deeper dive, see our [architecture docs](https://docs.paradedb.com/welcome/architecture) or [CMU Database Group talk](https://db.cs.cmu.edu/events/building-blocks-paradedb-philippe-noel/).

## Integrations

ParadeDB integrates with the tools you already use, with more on the way.

### ORMs & Frameworks

- [Drizzle](https://github.com/paradedb/drizzle-paradedb)
- [Django](https://github.com/paradedb/django-paradedb)
- [SQLAlchemy](https://github.com/paradedb/sqlalchemy-paradedb)
- [Rails](https://github.com/paradedb/rails-paradedb)
- [EF Core](https://github.com/paradedb/efcore-paradedb)
- More coming (Prisma, and others)

### AI Agents

- [Agent Skills](https://github.com/paradedb/agent-skills)
- [MCP Integration](https://docs.paradedb.com/documentation/getting-started/ai-agents)
- [Cursor Plugin](https://cursor.com/marketplace/parade-db)

### PaaS & Cloud Platforms

- [Railway](https://docs.paradedb.com/deploy/cloud-platforms/railway)
- [Render](https://docs.paradedb.com/deploy/cloud-platforms/render)
- [DigitalOcean](https://docs.paradedb.com/deploy/cloud-platforms/digitalocean)
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
