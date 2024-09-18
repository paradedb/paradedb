# **Contributing to ParadeDB**

Welcome! We're excited that you're interested in contributing to ParadeDB and want
to make the process as smooth as possible.

## Claiming and Working on Issues

### How to Claim an Issue

1. Before claiming an issue, ensure that:

   - It's not already assigned to someone else
   - There are no comments indicating ongoing work

2. To claim an unassigned issue, simply comment `/take` on the issue.
   This will automatically assign the issue to you.

### Unable to Make Progress?

If you find yourself unable to make progress on an assigned issue:

1. Unassign yourself:

   - Use the `unassign me` link at the top of the issue page

2. Seek help:

   - If you're stuck, don't hesitate to ask for help in the issue comments
   - Explain what you've tried and where you're having difficulties

3. Allow others to contribute:
   - By unassigning yourself, you open the opportunity for others to work on the issue

Remember, our goal is to maintain an efficient workflow and foster collaboration. If you're unable to continue work on an issue, it's best to unassign yourself promptly so that others can pick up where you left off.

## Technical Info

Before submitting a pull request, please review this document, which outlines what
conventions to follow when submitting changes. If you have any questions not covered
in this document, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ)
or via [email](mailto:support@paradedb.com).

### Development Workflow

ParadeDB is structured as a monorepo containing our Postgres extensions, our Docker setup, and our development tools for benchmarking and testing.

The development of our Postgres extensions is done via `pgrx`. For development instructions regarding a specific Postgres extension, please refer to the Development section of the README in the extension's subfolder.

The development of ParadeDB, which is the combination of our Postgres extensions and of community Postgres extensions packaged together, is done via Docker. If you are contributing to our Docker setup, we encourage you to use Docker Compose to build and test with the development file via `docker compose -f docker-compose.dev.yml up`.

### Pull Request Worfklow

All changes to ParadeDB happen through GitHub Pull Requests. Here is the recommended
flow for making a change:

1. Before working on a change, please check to see if there is already a GitHub issue open for that change.
2. If there is not, please open an issue first. This gives the community visibility into what you're working on and allows others to make suggestions and leave comments.
3. Fork the ParadeDB repo and branch out from the `dev` branch.
4. Install [pre-commit](https://pre-commit.com/) hooks within your fork with `pre-commit install` to ensure code quality and consistency with upstream.
5. Make your changes. If you've added new functionality, please add tests. We will not merge a feature without appropriate tests.
6. Open a pull request towards the `dev` branch. Ensure that all tests and checks pass. Note that the ParadeDB repository has pull request title linting in place and follows the [Conventional Commits spec](https://github.com/amannn/action-semantic-pull-request).
7. Congratulations! Our team will review your pull request.

### Documentation

ParadeDB's public-facing documentation is stored in the `docs` folder. If you are adding a new feature that requires new documentation, please add the documentation as part of your pull request. We will not merge a feature without appropriate documentation.

## Legal Info

### Contributor License Agreement

In order for us, Retake, Inc. (dba ParadeDB) to accept patches and other contributions from you, you need to adopt our ParadeDB Contributor License Agreement (the "**CLA**"). The current version of the CLA can be found [here](https://cla-assistant.io/paradedb/paradedb).

ParadeDB uses a tool called CLA Assistant to help us keep track of the CLA status of contributors. CLA Assistant will post a comment to your pull request indicating whether you have signed the CLA or not. If you have not signed the CLA, you will need to do so before we can accept your contribution. Signing the CLA is a one-time process, is valid for all future contributions to ParadeDB, and can be done in under a minute by signing in with your GitHub account.

If you have any questions about the CLA, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ) or via email at [legal@paradedb.com](mailto:legal@paradedb.com).

### License

By contributing to ParadeDB, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0](LICENSE) and as commercial software.
