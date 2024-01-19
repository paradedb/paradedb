<h1 align="center">
  <a href="https://paradedb.com"><img src="docs/logo/readme.svg" alt="ParadeDB" width="368px"></a>
<br>
</h1>

<p align="center">
  <b>Postgres for Search and Analytics</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Docs</a> &bull;
  <a href="https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ">Community</a> &bull;
  <a href="https://docs.paradedb.com/blog/">Blog</a> &bull;
  <a href="https://docs.paradedb.com/changelog/">Changelog</a>
</h3>

---

[![Publish ParadeDB](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml)
[![codecov](https://codecov.io/gh/paradedb/paradedb/graph/badge.svg?token=PI3TWD558R)](https://codecov.io/gh/paradedb/paradedb)
[![Docker Pulls](https://img.shields.io/docker/pulls/paradedb/paradedb)](https://hub.docker.com/r/paradedb/paradedb)
[![Artifact Hub](https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/paradedb)](https://artifacthub.io/packages/search?repo=paradedb)

[ParadeDB](https://paradedb.com) is an ElasticSearch alternative built on Postgres. We're building the features of ElasticSearch's product suite, starting with real-time search and analytics.

## Status

ParadeDB is currently in Public Beta. Star and watch this repository to get notified of updates.

### Roadmap

- [x] Search
  - [x] Full-text search with BM25 with [pg_bm25](https://github.com/paradedb/paradedb/tree/dev/pg_bm25#overview)
  - [x] Sparse vector search with [pg_sparse](https://github.com/paradedb/paradedb/tree/dev/pg_sparse#overview)
  - [x] Dense vector search with [pgvector](https://github.com/pgvector/pgvector#pgvector)
  - [x] Hybrid search
- [ ] Analytics
  - [x] Real-time analytical queries and Delta Lake storage with [pg_analytics](https://github.com/paradedb/paradedb/tree/dev/pg_analytics#overview)
  - [ ] High volume log ingest
  - [ ] Kafka ingest
  - [ ] Apache Iceberg storage
- [x] Self-hosted ParadeDB
  - [x] Docker image & [deployment instructions](https://docs.paradedb.com/deploy/aws)
  - [x] Kubernetes Helm chart & [deployment instructions](https://docs.paradedb.com/deploy/helm)
- [ ] Cloud Database
  - [ ] Cloud Marketplace Images

## Creating a ParadeDB Instance

### ParadeDB Cloud

ParadeDB Cloud is currently under development. To get notified when it becomes live, we invite you to join our
[waitlist](https://paradedb.typeform.com/to/jHkLmIzx).

### Self-Hosted

#### ParadeDB Docker Image

To install ParadeDB locally or on-premise, simply pull and run the latest Docker image:

```bash
docker run \
  -e POSTGRES_USER=<user> \
  -e POSTGRES_PASSWORD=<password> \
  -e POSTGRES_DB=<dbname> \
  -p 5432:5432 \
  -v paradedb-data:/var/lib/postgresql/data \
  -d \
  paradedb/paradedb:latest
```

The `-v` flag is optional, but recommended. It will persist the ParadeDB PostgreSQL data across restarts. Alternatively, you can clone this repo and run our `docker-compose.yml` file. By default, this will start the ParadeDB database at `http://localhost:5432`. Use `psql` to connect:

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

ParadeDB is licensed under the [GNU Affero General Public License v3.0](LICENSE). The `pg_sparse` extension is licensed under the [PostgreSQL License](pg_sparse/LICENSE).
