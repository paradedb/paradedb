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

[ParadeDB](https://paradedb.com) is a Postgres extension that brings full-text search and analytics directly inside Postgres.

- [x] [Full-Text Search](https://docs.paradedb.com/documentation/full-text/overview)
  - [x] [BM25 Scoring](https://docs.paradedb.com/documentation/sorting/score)
  - [x] [Tokenizers & Token Filters](https://docs.paradedb.com/documentation/tokenizers/overview)
  - [x] [Highlighting](https://docs.paradedb.com/documentation/full-text/highlight)
  - [x] [Top K](https://docs.paradedb.com/documentation/sorting/topk)
- [x] [Filtering](https://docs.paradedb.com/documentation/filtering)
- [x] [Aggregates](https://docs.paradedb.com/documentation/aggregates/overview)
  - [x] [Columnar Storage](https://docs.paradedb.com/documentation/indexing/columnar)
  - [x] [Bucket & Metrics](https://docs.paradedb.com/documentation/aggregates/overview)
  - [x] [Facets](https://docs.paradedb.com/documentation/aggregates/facets)
- [x] [JOINs](https://docs.paradedb.com/documentation/joins/overview)
- [x] [Performance Tuning](https://docs.paradedb.com/documentation/performance-tuning/overview)
- [ ] Vector Search (coming soon)
- [ ] Hybrid Search (coming soon)

Star and watch this repository to follow along. See our [current projects](https://github.com/paradedb/paradedb/projects?query=is%3Aopen) and [long-term roadmap](https://docs.paradedb.com/welcome/roadmap).

## How It Works

ParadeDB is a single Postgres extension built on:

- [pgrx](https://github.com/pgcentralfoundation/pgrx) — bridges Postgres and Rust
- [Tantivy](https://github.com/quickwit-oss/tantivy) — powers full-text search
- [Apache DataFusion](https://github.com/apache/datafusion) — handles OLAP processing

For a deeper dive, see our [architecture docs](https://docs.paradedb.com/welcome/architecture) or [CMU Database Group talk](https://db.cs.cmu.edu/events/building-blocks-paradedb-philippe-noel/).

## Integrations

ParadeDB integrates with the tools you already use, with more coming.

### ORMs & Frameworks

- [Django](https://github.com/paradedb/django-paradedb)
- [SQLAlchemy](https://github.com/paradedb/sqlalchemy-paradedb)
- [Rails](https://github.com/paradedb/rails-paradedb)
- More coming (Prisma, and others)

### AI Agents

- [Agent Skills](https://github.com/paradedb/agent-skills)
- [MCP Integration](https://docs.paradedb.com/welcome/ai-agents)

### PaaS & Cloud Platforms

- [Render](https://docs.paradedb.com/deploy/cloud-platforms/render)
- [Railway](https://docs.paradedb.com/deploy/cloud-platforms/railway)
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
