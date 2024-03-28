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
  <a href="https://blog.paradedb.com">Blog</a> &bull;
  <a href="https://docs.paradedb.com/changelog/">Changelog</a>
</h3>

---

[![Publish ParadeDB](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml/badge.svg)](https://github.com/paradedb/paradedb/actions/workflows/publish-paradedb.yml)
[![Docker Pulls](https://img.shields.io/docker/pulls/paradedb/paradedb)](https://hub.docker.com/r/paradedb/paradedb)
[![pg_analytics Deployments](https://img.shields.io/badge/20k-violet?label=pg_analytics%20deployments)]
[![pg_search Deployments](https://img.shields.io/badge/22k-green?label=pg_search%20deployments)]
[![Artifact Hub](https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/paradedb)](https://artifacthub.io/packages/search?repo=paradedb)

[ParadeDB](https://paradedb.com) is an Elasticsearch alternative built on Postgres. We're modernizing the features of Elasticsearch's product suite, starting with real-time search and analytics.

## Status

ParadeDB is currently in Public Beta. Star and watch this repository to get notified of updates.

### Roadmap

- [x] Search
  - [x] Full-text search with BM25 with [pg_search](https://github.com/paradedb/paradedb/tree/dev/pg_search#overview)
  - [x] Sparse vector search with [pg_sparse](https://github.com/paradedb/paradedb/tree/dev/pg_sparse#overview)
  - [x] Dense vector search with [pgvector](https://github.com/pgvector/pgvector#pgvector)
  - [x] Hybrid search
- [ ] Analytics
  - [x] Acclerated analytical queries and column-oriented storage with [pg_analytics](https://github.com/paradedb/paradedb/tree/dev/pg_analytics#overview)
  - [ ] External object store integrations (S3/Azure/GCS/HDFS)
  - [ ] External Apache Iceberg and Delta Lake support
  - [ ] High volume data/Kafka ingest
  - [ ] Non-Parquet file formats (Avro/ORC)
- [x] Self-Hosted ParadeDB
  - [x] Docker image based on [bitnami/postgresql](https://hub.docker.com/r/bitnami/postgresql) & [deployment instructions](https://docs.paradedb.com/deploy/aws)
  - [x] Kubernetes Helm chart & [deployment instructions](https://docs.paradedb.com/deploy/helm)
- [ ] Cloud Database
  - [ ] Managed cloud
  - [ ] Cloud Marketplace Images
  - [ ] Web-based SQL Editor
- [x] Specialized Workloads
  - [x] Support for geospatial data with [PostGIS](https://github.com/postgis/postgis)
  - [x] Support for cron jobs with [pg_cron](https://github.com/citusdata/pg_cron)

## Get Started

To get started, please visit our [documentation](https://docs.paradedb.com).

## Creating a ParadeDB Instance

### ParadeDB Cloud

ParadeDB Cloud is currently under development. To get notified when it becomes live, we invite you to join our [waitlist](https://paradedb.typeform.com/to/jHkLmIzx).

### Self-Hosted

#### ParadeDB Docker Image

To quickly get a ParadeDB instance up and running, simply pull and run the latest Docker image:

```bash
docker run --name paradedb paradedb/paradedb
```

This will start a ParadeDB instance with default user `postgres` and password `postgres`. You can then connect to the database using `psql`:

```bash
docker exec -it paradedb psql -U postgres
```

To install ParadeDB locally or on-premise, we recommend using our `docker-compose.yml` file. Alternatively, you can pass the appropriate environment variables to the `docker run` command, replacing the <> with your desired values:

```bash
docker run \
  --name paradedb \
  -e POSTGRESQL_USERNAME=<user> \
  -e POSTGRESQL_PASSWORD=<password> \
  -e POSTGRESQL_DATABASE=<dbname> \
  -e POSTGRESQL_POSTGRES_PASSWORD=<superuser_password> \
  -v paradedb_data:/bitnami/postgresql \
  -p 5432:5432 \
  -d \
  paradedb/paradedb:latest
```

This will start a ParadeDB instance with non-root user `<user>` and password `<password>`. The `superuser_password` will be associated with superuser `postgres` and is necessary for ParadeDB extensions to install properly.

The `-v` flag enables your ParadeDB data to persist across restarts in a Docker volume named `paradedb_data`. The volume needs to be writable by a user with `uid = 1001`, which is a security requirement of the Bitnami PostgreSQL Docker image. You can do so with:

```bash
sudo useradd -u 1001 <user>
sudo chown <user> </path/to/paradedb_data>
```

You can then connect to the database using `psql`:

```bash
docker exec -it paradedb psql -U <user> -d <dbname> -p 5432 -W
```

ParadeDB collects anonymous telemetry to help us understand how many people are using the project. You can opt out of telemetry by setting `PARADEDB_TELEMETRY` to `false` or by unsetting the variable.

#### ParadeDB Helm Chart

ParadeDB is also available for Kubernetes via our Helm chart. You can find our Helm chart in the [ParadeDB Helm Chart GitHub repository](https://github.com/paradedb/helm-charts) or download it directly from [Artifact Hub](https://artifacthub.io/packages/helm/paradedb/paradedb).

#### ParadeDB Extension(s)

To install the ParadeDB extension(s) manually within an existing self-hosted Postgres,
see the extension(s)' README. We strongly recommend using the ParadeDB Docker image,
which is optimized for running search in Postgres.

If you are self-hosting Postgres and are interested in ParadeDB, please [contact the ParadeDB team](mailto:hello@paradedb.com) and we'll be happy to help!

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

ParadeDB is licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software, with the exception of `pg_sparse` which is licensed under the [PostgreSQL License](pg_sparse/LICENSE).

For commercial licensing, please contact us at [sales@paradedb.com](mailto:sales@paradedb.com).

If you are an open-source project and would like to use ParadeDB under a different license, please contact us at [hello@paradedb.com](mailto:hello@paradedb.com).
