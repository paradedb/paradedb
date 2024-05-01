<h1 align="center">
  <img src="../docs/logo/pg_analytics.svg" alt="pg_analytics" width="500px">
<br>
</h1>

## Overview

`pg_lakehouse` is an extension that lets you query external data lakes and table formats directly from Postgres. Queries are pushed down
to the DataFusion query engine, which significantly accelerates query speeds.`pg_lakehouse` serves two purposes: to allow Postgres users to
run analytics over data lakes with zero data movement, and to seamlessly ingest data from data lakes into Postgres.

`pg_lakehouse` supports various object stores, table formats, and file formats.

### Supported Object Stores

- [x] Amazon S3
- [x] Local file system
- [ ] Azure Blob Storage (coming soon)
- [ ] Google Cloud Storage (coming soon)
- [ ] HDFS (coming soon)

### Supported File Formats

- [x] Parquet
- [x] CSV
- [x] JSON
- [x] Avro
- [ ] ORC (coming soon)

### Supported Table Formats

- [x] Deltalake
- [ ] Apache Iceberg (coming soon)

## Getting Started

TBD

## Query Acceleration

On its own, `pg_lakehouse` is only able to push down column projections, sorts, and limits to DataFusion. This means that queries containing
aggregates, joins, etc. are not fully accelerated.

You can solve this issue by installing `pg_analytics` alongside `pg_lakehouse`. `pg_analytics` intercepts the query and is able to push down most queries entirely to DataFusion.

```sql
\timing

-- Query speed without pg_analytics
SELECT COUNT(*) FROM hits;

CREATE EXTENSION pg_analytics;

-- Query speed with pg_analytics
```
