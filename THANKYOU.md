# ParadeDB Thank You Page

ParadeDB would not be possible without the support and assistance of other open-source
tools and the companies and communities behind them. On this page, we want to recognize
the most important open-source or otherwise free parts of our stack.

While the tools highlighted here are integral to our operations, we also
acknowledge the myriad of smaller packages that play a crucial role in our ecosystem.
To every contributor and maintainer out there, we are deeply thankful.

## Technologies

### PostgreSQL

[PostgreSQL](https://www.postgresql.org/) is an advanced, enterprise-class, and
open-source relational database system. With more than three decades of active development,
it has proven architecture and a strong reputation for reliability, data integrity,
and correctness. ParadeDB's core is built on PostgreSQL.

### PostgreSQL Extensions

[PostgreSQL extensions](https://pgxn.org/) are a rich ecosystem of add-ons that extend
the functionality of the core PostgreSQL database system. They enable a range of
advanced capabilities, from performance monitoring to geospatial indexing. ParadeDB
develops its own PostgreSQL extensions and integrates various open-source extensions
in our product. Special mention goes to [pgvector](https://github.com/pgvector/pgvector),
which we use to power part of our search capabilities. For a detailed list of the
extensions we use in ParadeDB, please refer to our Dockerfile(s).

### PGRX

[PGRX](https://github.com/pgcentralfoundation/pgrx) is a powerful toolset for
PostgreSQL extension development in Rust. It simplifies the process of creating,
testing, and packaging extensions, enabling developers to harness the performance
and safety guarantees of Rust within the PostgreSQL ecosystem. ParadeDB uses PGRX
for developing our own PostgreSQL extensions, and has drawn inspiration from ZomboDB,
the first PGRX extension and primary example, for the architecture of our own extensions.

### Tantivy

[Tantivy](https://github.com/quickwit-oss/tantivy) is a full-text search library
inspired by Apache Lucene, written entirely in Rust. ParadeDB uses Tantivy to power
part of our search functionalities.

### Docker

[Docker](https://www.docker.com) is a software platform that allows developers to
package and deploy applications inside containers. Containers are lightweight, portable,
and self-sufficient environments that can run on any operating system or cloud platform.
ParadeDB uses Docker to develop, package, and deploy our software.

### Kubernetes

[Kubernetes](https://kubernetes.io), also known as K8s, is an open-source system
for automating deployment, scaling, and management of containerized applications.
ParadeDB uses Kubernetes to deploy our software.
