# ParadeDB Thank You Page

ParadeDB would not be possible without the support and assistance of other open source
tools and the companies and communities behind them. On this page, we want to recognize
the most important open source or otherwise free parts of our stack.

While the tools highlighted here are integral to our operations, we also
acknowledge the myriad of smaller packages that play a crucial role in our ecosystem.
To every contributor and maintainer out there, we are deeply thankful.

## PostgreSQL

[PostgreSQL](https://www.postgresql.org/) is an advanced, enterprise-class, and
open source relational database system. With more than three decades of active development,
it has proven architecture and a strong reputation for reliability, data integrity,
and correctness. ParadeDB's core is built on PostgreSQL.

## pgvector

[pgvector](https://github.com/pgvector/pgvector) is an open source Postgres extension that enables
similarity search for Postgres. ParadeDB uses `pgvector` to power the vector search part of our
search capabilities.

## pgrx

[pgrx](https://github.com/pgcentralfoundation/pgrx) is a powerful toolset for
PostgreSQL extension development in Rust. It simplifies the process of creating,
testing, and packaging extensions, enabling developers to harness the performance
and safety guarantees of Rust within the PostgreSQL ecosystem. ParadeDB uses PGRX
for developing our own PostgreSQL extensions, and has drawn inspiration from [ZomboDB](https://github.com/zombodb/zombodb),
the first PGRX extension and primary example, for the architecture of our own extensions.

## Tantivy

[Tantivy](https://github.com/quickwit-oss/tantivy) is a full text search library
inspired by Apache Lucene, written entirely in Rust. ParadeDB uses Tantivy to power
part of our search functionalities.

## DuckDB

[DuckDB](https://github.com/duckdb/duckdb) is a high-performance analytical database system. It is designed to be fast, reliable, portable, and easy to use. ParadeDB uses DuckDB do vectorized query processing for columnar data.

## Docker

[Docker](https://www.docker.com) is a software platform that allows developers to
package and deploy applications inside containers. Containers are lightweight, portable,
and self-sufficient environments that can run on any operating system or cloud platform.
ParadeDB uses Docker to develop, package, and deploy our software.

## Kubernetes

[Kubernetes](https://kubernetes.io), also known as K8s, is an open source system
for automating deployment, scaling, and management of containerized applications.
ParadeDB uses Kubernetes to deploy our software.

## CloudNativePG

[CloudNativePG](https://github.com/cloudnative-pg/cloudnative-pg) is a PostgreSQL
operator for production-grade PostgreSQL clusters on Kubernetes. It covers the full
lifecycle of a highly available PostgreSQL database cluster with a primary/standby
architecture, using native streaming replication. ParadeDB uses CloudNativePG to
manage our PostgreSQL clusters.
