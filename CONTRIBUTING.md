# **Contributing to ParadeDB**

Welcome! We're excited that you're interested in contributing to ParadeDB and want
to make the process as smooth as possible.

## Technical Info

Before submitting a pull request, please review this document, which outlines what
conventions to follow when submitting changes. If you have any questions not covered
in this document, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ)
or via [email](support@paradedb.com).

### Development Workflow

ParadeDB is structured as a monorepo containing all the projects, PostgreSQL extension(s), and other
tools which together make ParadeDB. For development instructions regarding a specific project or Postgres extension,
please refer to the README in the project's subfolder. For developing ParadeDB itself as the combination
of all its subprojects, please see below.

All development of ParadeDB is done via Docker and Compose. Our Docker setup is split into three:

- The `docker-compose.dev.yml` file builds our `Dockerfile`, the ParadeDB production image with all its features and extensions enabled. It is used to develop and test ParadeDB Postgres extensions and features as part of the full ParadeDB image. It is also used to develop and test new features and extensions outside of those actively developed by ParadeDB (for instance, installing a new third-party open-source PostgreSQL extension). We recommend using it when developing new features beyond the ParadeDB extensions and subprojects.

- The `docker-compose.yml` file pulls the latest published ParadeDB image from DockerHub. It is used for hobby production deployments. We recommend using it to deploy ParadeDB in your own infrastructure.

### Pull Request Worfklow

All changes to ParadeDB happen through GitHub Pull Requests. Here is the recommended
flow for making a change:

1. Before working on a change, please check to see if there is already a GitHub
   issue open for that change.
2. If there is not, please open an issue first. This gives the community visibility
   into what you're working on and allows others to make suggestions and leave comments.
3. Fork the ParadeDB repo and branch out from the `dev` branch.
4. Install pre-commit hooks within your fork with `pre-commit install`, to ensure code quality and consistency with upstream.
5. Make your changes. If you've added new functionality, please add tests.
6. Open a pull request towards the `dev` branch. Ensure that all tests and checks
   pass. Note that the ParadeDB repository has pull request title linting in place
   and follows the [Conventional Commits spec](https://github.com/amannn/action-semantic-pull-request).
7. Congratulations! Our team will review your pull request.

### Documentation

ParadeDB's public-facing documentation is stored in the `docs` folder. If you are
adding a new feature that requires new documentation, please open a separate pull
request containing changes to the documentation only. Once your main pull request
is merged, the ParadeDB team will review and eventually merge your documentation
changes as well.

## Legal Info

### Contributor License Agreement

In order for us, Retake, Inc. (dba ParadeDB) to accept patches and other contributions from you, you need to adopt our ParadeDB Contributor License Agreement (the "**CLA**"). The current version of the CLA can be found [here](https://cla-assistant.io/paradedb/paradedb).

ParadeDB uses a tool called CLA Assistant to help us keep track of the CLA status of contributors. CLA Assistant will post a comment to your pull request, indicating whether you have signed the CLA or not. If you have not signed the CLA, you will need to do so before we can accept your contribution. Signing the CLA is a one-time process, is valid for all future contributions to ParadeDB, and can be done in under a minute by signing in with your GitHub account.

If you have any questions about the CLA, please reach out to us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-2lkzdsetw-OiIgbyFeiibd1DG~6wFgTQ) or via email at [legal@paradedb.com](mailto:legal@paradedb.com).

### License

By contributing to ParadeDB, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0](LICENSE).
