# Stressgres

Stressgres is a stress-testing tool for ParadeDB and standard PostgreSQL, featuring both a text UI and an automated headless mode. We used it for local development and in CI to replicate and test against representative customer workloads, called suites.

## Quickstart

- Run the interactive UI against a suite:

```bash
cargo run -- ui suites/vanilla-postgres.toml
```

- Run headless mode with logging:

```bash
cargo run -- headless suites/vanilla-postgres.toml --runtime=300000 --log-file=logs/test.log
```

Suites are TOML files in `suites/`. The `vanilla-postgres.toml` suite exercises baseline Postgres features and works with any PostgreSQL-compatible server.

## Docker

To run Stressgres from within Docker, use:

```bash
docker run --rm paradedb/stressgres:latest cargo run -- headless suites/vanilla-postgres.toml
```

The source, including all suites, are loaded in the Docker image. The image prebuilds Stressgres and can run in air-gapped environments like Antithesis.

For an interactive shell:

```bash
docker run -d --name stressgres paradedb/stressgres:latest
docker exec -it stressgres bash
```

### Docker Hub

To publish the Stressgres image to Docker Hub, trigger a workflow dispatch of the `Publish Stressgres (Docker) from within the Actions tab. This is useful to get updated Stressgres binaries to our BYOC end-to-end testing framework.

## Antithesis

Antithesis is a deterministic simulation testing (DST) tool. Stressgres, via the Docker image, is able to run within Antithesis to execute suites in a fully deterministic environment. To execute a Stressgres suite within Antithesis, it needs to have its own `singleton_driver_` file defined within the `suites/antithesis/` folder. For more information on how Antithesis singleton drivers work, please refer to [the Antithesis documentation](https://antithesis.com/docs/getting_started/setup_k8s/#basic-test-template).

To add a new suite:

- Create the corresponding singleton driver

- Trigger a release of the Docker image to the Antithesis registry via the `Test pg_search (Antithesis)` workflow. This workflow builds and publishes the latest commit ParadeDB and Stressgres Docker images to Antithesis, and triggers a test run.

If it behaves as desired, merge your new singleton driver to `main`. The new suite will then be added to the nightly Antithesis runs.

### Connection Strings

To facilitate testing with Antithesis, we deploy the ParadeDB CloudNativePG cluster with a manifest that hardcodes a dummy password. This ensures we can modify the Stressgres suite `connection_string` without needing to extract passwords dynamically.

The manifests can be found at `docker/manifests/paradedb.yaml` in `paradedb/paradedb`/`paradedb/paradedb-enterprise` and the password used in the singleton driver files must match.
