# **Contributing to ParadeDB**

Welcome! We're excited that you're interested in contributing to ParadeDB and want to make the process as smooth as possible.

## Technical Info

Before submitting a pull request, please review this document, which outlines what conventions to follow when submitting changes. If you have any questions not covered in this document, please reach out to us in the [ParadeDB Community Slack](https://paradedb.com/slack) or via [email](mailto:support@paradedb.com).

### Selecting GitHub Issues

All external contributions should be associated with a GitHub issue. If there is no open issue for the bug or feature that you'd like to work on, please open one first. When selecting an issue to work on, we recommend focusing on issues labeled `good first issue`.

Ideal issues for external contributors include well-scoped, individual features (e.g. adding support for a new tokenizer) as those are less likely to conflict with our general development process. We welcome small documentation contributions that accompany a feature, correct wrong information or fix typos, but will not accept "general improvement" documentation PRs.

### Claiming GitHub Issues

This repository has a workflow to assign issues to new contributors automatically. This ensures that you don't need approval
from a maintainer to pick an issue.

1. Before claiming an issue, ensure that:

- It's not already assigned to someone else
- There are no comments indicating ongoing work

2. To claim an unassigned issue, comment `/take` on the issue. This will automatically assign the issue to you.

If you find yourself unable to make progress, don't hesitate to seek help in the issue comments or the [ParadeDB Community Slack](https://paradedb.com/slack). If you no longer wish to
work on the issue(s) you self-assigned, please use the `unassign me` link at the top of the issue(s) page to release it.

### Development Workflow

ParadeDB is a Postgres extension, `pg_search`, written in Rust and packaged either as a standalone binary or as a Docker image. The development of our Postgres extension is done via `pgrx`.

For instructions on setting up your development environment, building, and running `pg_search` locally, see the [pg_search README](/pg_search/README.md).

### Pull Request Workflow

All changes to ParadeDB happen through GitHub Pull Requests. Here is the recommended flow for making a change:

1. Before working on a change, please check if there is already a GitHub issue open for it.
2. If there is not, please open an issue first. This gives the community visibility into your work and allows others to make suggestions and leave comments.
3. Fork the ParadeDB repo and branch out from the `main` branch.
4. Install [prek](https://github.com/j178/prek) hooks within your fork with `prek install` to ensure code quality and consistency with upstream.
5. Make your changes. If you've added new functionality, please add tests. We will not merge a feature without appropriate tests.
6. Open a pull request towards the `main` branch. Ensure that all tests and checks pass. Note that the ParadeDB repository has pull request title linting in place and follows the [Conventional Commits spec](https://github.com/amannn/action-semantic-pull-request).
7. Congratulations! Our team will review your pull request.

### Documentation

ParadeDB's public-facing documentation is stored in the `docs` folder. If you are adding a new feature that requires new documentation, please add the documentation as part of your pull request. We will not merge a feature without appropriate documentation.

### Testing

ParadeDB has four main categories of tests. For a full overview of how and when to use them, please see their respective documentation:

#### 1. pg regress tests

Located in `pg_search/tests/pg_regress`.

- **Purpose:** These are for output / golden testing, and are useful when the output is small enough that you can inspect it visually to determine correctness.
- **Running:** Run them with `cargo pgrx regress -p pg_search --auto -- pg18 one_file_name`. There is no need to manually install the extension: it is handled automatically.
- **Details:** See [`pg_search/tests/pg_regress/README.md`](pg_search/tests/pg_regress/README.md) for more details.

#### 2. Integration tests

Located in the `tests/` directory.

- **Purpose:** These tests run outside the Postgres process as a client. They should be used to assert things when output is too complicated to visually inspect, or is non-deterministic (such as property testing).
- **Running:** Since these run outside the process, they need the extension to already be installed. Run them with `cargo test -p tests -- a_specific_method_to_run`.
- **Details:** See [`tests/README.md`](tests/README.md) for more details.

#### 3. Unit tests

Located in the `pg_search/src` directory.

- **Purpose:** They are either:
  - **Unit tests without Postgres** if they are not marked `#[pg_test]`.
  - **Unit tests which run in Postgres as UDFs** if they are marked `#[pg_test]`. These use all of Postgres APIs via `pgrx`.
- **Running:** Run them with `cargo test -p pg_search -- a_specific_method_to_run`. There is no need to pre-install the extension for `#[pg_test]` annotated tests (the annotation automatically handles it).
- **Details:** See [`pg_search/README.md`](pg_search/README.md#testing) for more details.

#### 4. Stress tests (Stressgres)

Located in the `stressgres/` directory.

- **Purpose:** Replicate representative customer workloads against ParadeDB (or vanilla Postgres) to surface concurrency, correctness, and performance regressions that don't show up in shorter-lived tests. Also the entry point for Antithesis deterministic simulation runs.
- **Running:** Run a suite interactively with `cargo run -- ui suites/vanilla-postgres.toml`, or headlessly with `cargo run -- headless suites/vanilla-postgres.toml --runtime=300000`.
- **Details:** See [`stressgres/README.md`](stressgres/README.md) for more details.

## Legal Info

### Contributor License Agreement

In order for us, ParadeDB, Inc., to accept patches and other contributions from you, you need to adopt our ParadeDB Contributor License Agreement (the "**CLA**"). The current version of the CLA can be found on the [CLA Assistant website](https://cla-assistant.io/paradedb/paradedb).

ParadeDB uses a tool called CLA Assistant to help us track contributors' CLA status. CLA Assistant will post a comment to your pull request indicating whether you have signed the CLA. If you have not signed the CLA, you must do so before we can accept your contribution. Signing the CLA is a one-time process, is valid for all future contributions to ParadeDB, and can be done in under a minute by signing in with your GitHub account.

If you have any questions about the CLA, please reach out to us in the [ParadeDB Community Slack](https://paradedb.com/slack) or via email at [legal@paradedb.com](mailto:legal@paradedb.com).

### License

By contributing to ParadeDB, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software.
