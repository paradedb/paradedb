## Overview 

`pg_lakehouse` is an extension that lets you query external data lakes and table formats directly from Postgres. Queries are pushed down
to the DataFusion query engine, which offers vectorized, parallelized execution. 

`pg_lakehouse` is designed to work seamlessly across various object stores, table formats, and file formats.

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

On its own, `pg_lakehouse` often does not push down the entire query to DataFusion. 
