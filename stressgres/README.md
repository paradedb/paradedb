# Stressgres

Stressgres is a stress-testing tool for ParadeDB and standard PostgreSQL, featuring both a text UI and an automated headless mode.

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

The source, including all suites, are loaded in the Docker image. The image prebuilds Stressgres and can run in air-gapped environment, like within Antithesis.

For an interactive shell:

```bash
docker run -d --name stressgres paradedb/stressgres:latest
docker exec -it stressgres bash
```

## Releases

To create a release:

- Update the version number in `Cargo.toml` and run `cargo build` to refresh the lockfile.

- Create and push a new Git tag that matches the new `Cargo.toml` version:

```bash
git tag v0.3.0
git push origin v0.3.0
```

- From the Actions tab, trigger the build of a new Stressgres Docker image to Docker Hub, and optionally to Antithesis, by selecting the desired workflow and running a manual `workflow_dispatch` with the version of the tag, without the leading `v`, that you just created (e.g. `0.3.0`) as input.

## Antithesis

Antithesis is a deterministic simulation testing (DST) tool. Stressgres, via the Docker image, is able to run within Antithesis to execute suites in a fully deterministic environment. To execute a Stressgres suite within Antithesis, it needs to have its own `singleton_driver_` file defined within the `suites/antithesis/` folder. For more information on how Antithesis singleton drivers work, please refer to [the Antithesis documentation](https://antithesis.com/docs/getting_started/setup_k8s/#basic-test-template).

To add a new suite:

- Create the corresponding singleton driver

- Trigger a release of the Docker image to the Antithesis registry

The new suite will be executed in the next Antithesis run.

### Connection Strings

To facilitate testing with Antithesis, we deploy the ParadeDB CloudNativePG cluster with a manifest that hardcodes a dummy password. This ensures we can modify the Stressgres suite `connection_string` without needing to extract passwords dynamically.

The manifests can be found at `docker/manifests/paradedb.yaml` in `paradedb/paradedb`/`paradedb/paradedb-enterprise` and the password used in the singleton driver files must match.
