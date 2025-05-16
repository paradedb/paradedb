# **Contributing to ParadeDB**

Welcome! We're excited that you're interested in contributing to ParadeDB and want to make the process as smooth as possible.

## Technical Info

Before submitting a pull request, please review this document, which outlines what
conventions to follow when submitting changes. If you have any questions not covered
in this document, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-32abtyjg4-yoYoi~RPh9MSW8tDbl0BQw)
or via [email](mailto:support@paradedb.com).

### Claiming GitHub Issues

This repository has a workflow to assign issues to new contributors automatically. This ensures that you don't need approval
from a maintainer to pick an issue.

1. Before claiming an issue, ensure that:

- It's not already assigned to someone else
- There are no comments indicating ongoing work

2. To claim an unassigned issue, comment `/take` on the issue. This will automatically assign the issue to you.

If you find yourself unable to make progress, don't hesitate to seek help in the issue comments or the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-32abtyjg4-yoYoi~RPh9MSW8tDbl0BQw). If you no longer wish to
work on the issue(s) you self-assigned, please use the `unassign me` link at the top of the issue(s) page to release it.

### Development Workflow

ParadeDB is a Postgres extension, `pg_search`, written in Rust and packaged either as a standalone binary or as a Docker image. The development of our Postgres extension is done via `pgrx`. Please review the Development section of the [pg_search README](/pg_search/README.md).

### Pull Request Workflow

All changes to ParadeDB happen through GitHub Pull Requests. Here is the recommended
flow for making a change:

1. Before working on a change, please check if there is already a GitHub issue open for it.
2. If there is not, please open an issue first. This gives the community visibility into your work and allows others to make suggestions and leave comments.
3. Fork the ParadeDB repo and branch out from the `dev` branch.
4. Install [pre-commit](https://pre-commit.com/) hooks within your fork with `pre-commit install` to ensure code quality and consistency with upstream.
5. Make your changes. If you've added new functionality, please add tests. We will not merge a feature without appropriate tests.
6. Open a pull request towards the `dev` branch. Ensure that all tests and checks pass. Note that the ParadeDB repository has pull request title linting in place and follows the [Conventional Commits spec](https://github.com/amannn/action-semantic-pull-request).
7. Congratulations! Our team will review your pull request.

### Documentation

ParadeDB's public-facing documentation is stored in the `docs` folder. If you are adding a new feature that requires new documentation, please add the documentation as part of your pull request. We will not merge a feature without appropriate documentation.

## Legal Info

### Contributor License Agreement

In order for us, ParadeDB, Inc., to accept patches and other contributions from you, you need to adopt our ParadeDB Contributor License Agreement (the "**CLA**"). The current version of the CLA can be found [here](https://cla-assistant.io/paradedb/paradedb).

ParadeDB uses a tool called CLA Assistant to help us track contributors' CLA status. CLA Assistant will post a comment to your pull request indicating whether you have signed the CLA. If you have not signed the CLA, you must do so before we can accept your contribution. Signing the CLA is a one-time process, is valid for all future contributions to ParadeDB, and can be done in under a minute by signing in with your GitHub account.

If you have any questions about the CLA, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-32abtyjg4-yoYoi~RPh9MSW8tDbl0BQw) or via email at [legal@paradedb.com](mailto:legal@paradedb.com).

### License

By contributing to ParadeDB, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software.
