<h1 align="center">
  <a href="https://paradedb.com"><img src="docs/logo/readme.svg" alt="ParadeDB" width="368px"></a>
<br>
</h1>

<p align="center">
    <b>PostgreSQL for Search</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Cloud</a> &bull;
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Documentation</a> &bull;
  <a href="https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ">Community</a>
</h3>

---

[![Benchmark ParadeDB](https://github.com/paradedb/paradedb/actions/workflows/benchmark-paradedb.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/benchmark-paradedb.yml)
[![Publish ParadeDB](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml)
[![codecov](https://codecov.io/gh/paradedb/paradedb/graph/badge.svg?token=PI3TWD558R)](https://codecov.io/gh/paradedb/paradedb)
[![Artifact Hub](https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/paradedb)](https://artifacthub.io/packages/search?repo=paradedb)

[ParadeDB](https://paradedb.com) is an ElasticSearch alternative built on PostgreSQL,
engineered for lightning-fast full text, similarity, and hybrid search.

It offers the most comprehensive, Postgres-native search features of any Postgres
database, so you don't need to glue cumbersome services like a search engine or
vector database on top.

## Key Benefits

- ⚡ **Speed**: ParadeDB is built in Rust on top of PostgreSQL and Tantivy,
  a Rust-based implementation of Apache Lucene. See our benchmarks [here](./benchmarks/README.md).

- 🌿 **Simplicity**: Consolidate your database and search engine
  into a single system, so you don't need to worry about keeping separate services
  in sync.

- 🐘 **SQL First**: Write search queries in SQL with ACID transactions.

- 🚀 **Scalability**: Scale to millions of rows with support for distributed
  search, high availability, backups, and point-in-time-recovery.

### Roadmap

- [ ] Search
  - [x] Full-text search with BM25 with [pg_bm25](https://github.com/paradedb/paradedb/tree/dev/pg_bm25#overview)
  - [x] Similarity search with [pgvector](https://github.com/pgvector/pgvector#pgvector)
  - [x] Sparse vector similarity search with [pg_sparse](https://github.com/paradedb/paradedb/tree/dev/pg_sparse#overview)
  - [x] Hybrid search with [pg_search](https://github.com/paradedb/paradedb/tree/dev/pg_search#overview)
  - [x] Real-time search
  - [ ] Faceted search
  - [ ] Distributed search (horizontal scaling/sharding)
  - [ ] Generative search
  - [ ] Multimodal search
- [x] Self-hosting
  - [x] Docker image & [deployment instructions](https://docs.paradedb.com/deploy/aws)
  - [x] Kubernetes Helm chart & [deployment instructions](https://docs.paradedb.com/deploy/helm)
- [ ] Cloud Database
  - [x] Managed cloud
  - [x] Self-serve cloud
  - [ ] Public Cloud (AWS, GCP, Azure) Marketplace Images
  - [x] High availability
- [ ] Web-based SQL Editor

## Creating a ParadeDB Instance

### ParadeDB Cloud

Try [ParadeDB Cloud](https://dashboard.paradedb.com) to get started with a free, managed ParadeDB instance.
Then connect to it with any Postgres client.

### Self-Hosted

#### ParadeDB Docker Image

To install ParadeDB locally or on-premise, simply pull and run the latest Docker image:

```bash
docker run \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

Alternatively, you can clone this repo and run our `docker-compose.yml` file. By default, this will start the ParadeDB database at `http://localhost:5432`. Use `psql` to connect:

```bash
psql -h <hostname> -U <user> -d <dbname> -p 5432 -W
```

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt-out of telemetry by adding `-e TELEMETRY=false` (or unsetting the variable) to your `docker run` command, or by setting `TELEMETRY: false` in the `docker-compose.yml` file.

#### ParadeDB Helm Chart

ParadeDB is also available for Kubernetes via our Helm chart. You can find our Helm chart in the [ParadeDB Helm Chart GitHub repository](https://github.com/paradedb/helm-charts) or download it directly from [Artifact Hub](https://artifacthub.io/packages/helm/paradedb/paradedb).

#### ParadeDB Extension(s)

To install the ParadeDB extension(s) manually within an existing self-hosted Postgres,
see the extension(s)' README. We strongly recommend using the ParadeDB Docker image,
which is optimized for running search in Postgres.

If you are self-hosting Postgres and are interested in ParadeDB, please [contact the ParadeDB team](mailto:hello@paradedb.com) and we'll be happy to help!

## Getting Started

To get started using ParadeDB, please follow the [quickstart guide](https://docs.paradedb.com/quickstart)!

## Documentation

You can find the complete documentation for ParadeDB at [docs.paradedb.com](https://docs.paradedb.com).

## Support

If you're missing a feature or have found a bug, please open a
[GitHub Issue](https://github.com/paradedb/paradedb/issues/new/choose).

To get community support, you can:

- Post a question in the [ParadeDB Slack Community](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ)
- Ask for help on our [GitHub Discussions](https://github.com/paradedb/paradedb/discussions)

If you need commercial support, please [contact the ParadeDB team](mailto:sales@paradedb.com).

## Contributing

We welcome community contributions, big or small, and are here to guide you along
the way. To get started contributing, check our [first timer issues](https://github.com/paradedb/paradedb/labels/good%20first%20issue)
or message us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ). Once you contribute, ping us in Slack and we'll send you some ParadeDB swag!

For more information on how to contribute, please see our
[Contributing Guide](/CONTRIBUTING.md).

This project is released with a [Contributor Code of Conduct](/CODE_OF_CONDUCT.md).
By participating in this project, you agree to follow its terms.

Thank you for helping us make ParadeDB better for everyone :heart:.

## License

ParadeDB is licensed under the [GNU Affero General Public License v3.0](LICENSE), except for `pg_sparse` which is licensed under the [PostgreSQL License](pg_sparse/LICENSE).
