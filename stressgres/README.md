# Stressgres

Stressgres is a stress-testing tool for ParadeDB and standard PostgreSQL, featuring both a text UI and an automated headless mode. We use it for local development and in CI to replicate and test against representative customer workloads, called suites.

## Quickstart

- Run the interactive UI against a suite:

```bash
cargo run -p stressgres -- ui stressgres/suites/single-node-planner-paths.toml
```

- Run headless mode with logging:

```bash
cargo run -p stressgres -- headless stressgres/suites/single-node-planner-paths.toml --runtime=300000 --log-file=logs/test.log
```

- Run headless mode tolerating transient database faults (e.g. under Antithesis)

```bash
cargo run -p stressgres -- headless stressgres/suites/single-node-planner-paths.toml --runtime=300000 --reconnect-grace=30000
```

- Run a suite against a throwaway Postgres cluster built from a given `pg_config`:

```bash
cargo run -p stressgres -- auto /path/to/pg_config stressgres/suites/single-node-planner-paths.toml /tmp/stressgres-data --runtime 300000
```

Suites are TOML files in `stressgres/suites/`. Each suite describes a planner workload or PostgreSQL topology exercised by Stressgres.

A job can verify its query plan once whenever Stressgres opens a connection. Use a
string for one required fragment or an array when several nodes must be present;
the checks are case-insensitive and run after `on_connect` settings:

```toml
[[jobs]]
on_connect = "SET max_parallel_workers_per_gather = 0"
sql = "SELECT id FROM test WHERE message ||| 'beer' LIMIT 10"
sql_plan_contains = ["TopKScanExecState", "Custom Scan"]
```

Suites can compose a reusable workload with a reusable topology. Workloads own
schema setup and queries; topologies own servers, replication, logical `read` and
`write` destinations, and polling-interval scaling. A suite itself can therefore
be only two references:

```toml
workload = "workloads/background-merge.toml"
topology = "topologies/logical-replication.toml"
```

Topology files declare capabilities and route mappings. Workloads can use
`requires` to prevent combinations that would run but test the wrong behavior:

```toml
capabilities = ["indexed_writes", "logical_replication"]

[routes.read]
destinations = ["SubscriberA", "SubscriberB"]
refresh_scale = 2.0

# In a workload file:
requires = ["indexed_writes"]
```

The manifests under `stressgres/suites/matrix/` exercise every compatible pairing
of the reusable workloads with single-node, logical-replication, and
multi-subscriber topologies. Topology-specific race and asymmetric
vanilla-publisher suites remain standalone because composing them with arbitrary
layouts would change the behavior they are intended to test.

## Docker

To run Stressgres from within Docker, use:

```bash
docker run --rm paradedb/stressgres:latest /symbols/stressgres headless stressgres/suites/single-node-planner-paths.toml
```

The source, including all suites, is included in the Docker image. The image prebuilds Stressgres at `/symbols/stressgres` and can run in air-gapped environments like Antithesis.

For an interactive shell:

```bash
docker run -d --name stressgres paradedb/stressgres:latest
docker exec -it stressgres bash
```

### Docker Hub

To publish the Stressgres image to Docker Hub, trigger a workflow dispatch of `Publish Stressgres (Docker)` from the Actions tab. This is useful to get updated Stressgres binaries to our BYOC end-to-end testing framework.

## Antithesis

Antithesis is a deterministic simulation testing (DST) tool. Stressgres runs inside Antithesis through the Docker image. Each suite has a `first_<suite>.sh` setup script under `stressgres/suites/antithesis/`; the shared `singleton_driver_stressgres.sh` then runs the selected workload under fault injection. For more information on Antithesis test commands, see [the Antithesis documentation](https://antithesis.com/docs/test_templates/first_test/).

To add a new suite:

- Create the corresponding `first_<suite>.sh` setup script.

- Trigger a release of the Docker image to the Antithesis registry via the `Test pg_search (Antithesis)` workflow. This workflow builds and publishes the latest commit ParadeDB and Stressgres Docker images to Antithesis, and triggers a test run.

If it behaves as desired, merge your new singleton driver to `main`. The new suite will then be added to the nightly Antithesis runs.

### Connection Strings

To facilitate testing with Antithesis, we deploy the ParadeDB CloudNativePG cluster with a manifest that hardcodes a dummy password. This ensures we can modify the Stressgres suite `connection_string` without needing to extract passwords dynamically.

The manifests can be found at `docker/manifests/antithesis-paradedb.yaml` in `paradedb/paradedb`/`paradedb/paradedb-enterprise` and the password used in the singleton driver files must match.
