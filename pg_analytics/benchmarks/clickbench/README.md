# ParadeDB

ParadeDB is an alternative to ElasticSearch built on PostgreSQL.

- [Homepage](https://paradedb.com)
- [GitHub](https://github.com/paradedb/paradedb)

## Running the benchmark

### Local

The benchmarks are configured to run locally via `cargo pgrx bench` within `pg_analytics`,

### Official

The benchmark has been configured for a `c6a.4xlarge` running Ubuntu 22.04 and can be run without attendance.

```bash
export FQDN=ec2-127-0-0-1.compute-1.amazonaws.com
scp -i ~/.ssh/aws.pem *.sh *.sql ubuntu@$FQDN:~
ssh -i ~/.ssh/aws.pem ubuntu@$FQDN ./benchmark.sh
```

List of things we can do to tune, based on postgresql-tuned and datafusion:

- Check the datafusion one, and do the same ones onc we have them
- Check the postgresql-tuned, and do the same one
- Check the clickbench full README
