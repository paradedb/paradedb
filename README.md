<h1 align="center">
  <img src="docs/logo/readme.svg" alt="ParadeDB" width="368px"></a>
<br>
</h1>

<p align="center">
    <b>PostgreSQL for Search</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Documentation</a> &bull;
  <a href="https://paradedb.com/blog">Blog</a> &bull;
  <a href="https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ">Community</a>
</h3>

---

[![Publishing](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb-to-dockerhub.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb-to-dockerhub.yml)

[ParadeDB](https://paradedb.com) is an ElasticSearch alternative built on PostgreSQL.

It offers the most comprehensive, Postgres-native search features of any Postgres
database, so you don't need to glue cumbersome services like a search engine or
vector database on top.

ParadeDB offers four primary benefits to users:

- **Speed**: ParadeDB is built in Rust on top of PostgreSQL and Tantivy, a
  Rust-based implementation of Apache Lucene. On average, ParadeDB queries are
  2x faster than ElasticSearch. See our [benchmarks](https://github.com/paradedb/paradedb/tree/dev/benchmarks/README.md)
  (coming soon!).

- **Simplicity**: Consolidate your database and search engine
  into a single system, so you don't need to worry about keeping separate services
  in sync.

- **SQL First**: Write search queries in SQL with ACID transactions.

- **Scalability**: Scale to millions of rows with support for distributed search,
  high availability, backups, and point-in-time-recovery (coming soon!).

## Status

ParadeDB is still under active development and is not yet ready to use
in production. We're aiming to be ready by the end of September 2023.

We are currently in Private Alpha. Star & watch this repo to get notified of
major updates.

### Roadmap

- [ ] Search
  - [x] Full-text search with BM25
  - [ ] Faceted search
  - [x] Similarity search
  - [ ] Hybrid search
  - [ ] Distributed search
  - [x] Real-time search
  - [ ] Generative search
  - [ ] Multimodal search
- [ ] Cloud Database
  - [ ] Managed cloud
  - [ ] Self-serve cloud
  - [ ] Public Cloud (AWS, GCP, Azure) Marketplace Images
  - [ ] High availability
- [ ] Web-based SQL Editor

## Installation

To install locally or on-premise, simply run the latest Docker image:

```bash
docker run \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -p 5432:5432 \
  paradedb/paradedb:latest
```

By default, this will start the ParadeDB database at `http://localhost:5432`.

## Get Started

Follow the [quickstart guide](https://docs.paradedb.com/quickstart) to get started!

## Documentation

You can find the complete documentation for ParadeDB at [docs.paradedb.com](https://docs.paradedb.com).

## Support

If you're missing a feature or have found a bug, please open a
[GitHub Issue](https://github.com/paradedb/paradedb/issues/new/choose).

To get community support, you can:

- Post a question on the [ParadeDB Slack Community](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ)
- Ask for help on our [GitHub Discussions](https://github.com/paradedb/paradedb/discussions)

If you need commercial support, please [contact](mailto:sales@paradedb.com) the
ParadeDB team.

## Contributing

We welcome community contributions, big or small, and are here to guide you along
the way. To get started contributing, check our [first timer issues](https://github.com/paradedb/paradedb/labels/good%20first%20issue)
or message us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ).
Once you contribute, ping us in Slack and we'll send you some ParadeDB swag!

If you're missing a feature or have found a problem with ParadeDB, please open a
[GitHub issue](https://github.com/paradedb/paradedb/issues/new/choose).

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

This project is released with a [Contributor Code of Conduct](https://github.com/paradedb/paradedb/blob/stable/CODE_OF_CONDUCT.md).
By participating in this project, you agree to follow its terms.

Thank you for helping us make ParadeDB better for everyone :heart:

### Current Contributors

<a href="https://github.com/paradedb/paradedb/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=paradedb/paradedb" />
</a>

## License

ParadeDB is licensed under the [Elastic License 2.0](LICENSE). Our goal with
choosing ELv2 is to enable our users to use and contribute to ParadeDB freely,
while enabling us to continue investing in our community, project and product.
