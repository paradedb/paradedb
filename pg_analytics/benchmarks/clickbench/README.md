# ParadeDB

ParadeDB is an alternative to ElasticSearch built on PostgreSQL.

- [Homepage](https://paradedb.com)
- [GitHub](https://github.com/paradedb/paradedb)

## Running the benchmark

### Local

The benchmarks are configured to run locally via `cargo clickbench` from the `pg_analytics/` directory,

### Official

The benchmark has been configured for a `c6a.4xlarge` running Ubuntu 22.04 and can be run without attendance.

```bash
export FQDN=ec2-127-0-0-1.compute-1.amazonaws.com
scp -i ~/.ssh/aws.pem *.sh *.sql ubuntu@$FQDN:~
ssh -i ~/.ssh/aws.pem ubuntu@$FQDN ./benchmark.sh
```
