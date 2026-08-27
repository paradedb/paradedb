# ParadeDB Release Process

We use a single branch, `main`, for our development. Features are built on separate branches and merged into `main` via pull requests.

ParadeDB uses a **fragment-based workflow** for release artifacts:

- PRs that introduce user-facing changes add a changelog fragment to `docs/changelog/unreleased/<PR>.<category>.mdx`.
- PRs that modify extension DDL/schema add a SQL migration fragment to `pg_search/sql/unreleased/<PR>.<description>.sql`.

At release time, the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) automatically assembles these fragments into the versioned SQL upgrade script and changelog page.

## Release Types

Releases must always be triggered **from the branch being released** (e.g., `main` for a minor or beta release, or a stable branch for patches).

| Type          | Description                                                                                       |
| ------------- | ------------------------------------------------------------------------------------------------- |
| **Minor**     | Triggered from the `main` branch.                                                                 |
| **Patch**     | A patch bump off an existing tag (e.g., `v1.4.0 → v1.4.1`) from a stable branch (e.g., `0.25.x`). |
| **Beta (RC)** | Marked with `beta: true`. Produces a prerelease tag like `vX.Y.Z-rc.N`.                           |

> **Note:** Minor and patch releases publish Docker images for all supported PostgreSQL major versions and prebuilt extension binaries for all supported platforms. Beta releases publish only the PostgreSQL 18 Docker image and the Debian 13 packages required to build it; the remaining prebuilt extension binaries are skipped.

## Workflow Inputs

| Input     | Type    | Default | Description                                                                                      |
| --------- | ------- | ------- | ------------------------------------------------------------------------------------------------ |
| `version` | string  | `""`    | Target release version in semver format (e.g., `1.2.3` or `1.2.3-rc.1` for beta releases).       |
| `beta`    | boolean | `false` | If `true`, creates a beta release (e.g., `vX.Y.Z-rc.N`) and marks it as a pre-release in GitHub. |

## Triggering a Release

### Minor & Beta Releases

To publish a minor or beta release from `main`:

1. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
2. Click **Run workflow**, select `main` as the release branch, and provide the target `version` (e.g. `0.26.0` or `0.26.0-rc.1` with `beta: true`).
3. Click **Run workflow** to start the job.
4. The workflow will automatically:
   - Run `scripts/assemble_release_fragments.sh all` to assemble `pg_search/sql/unreleased/*.sql` into `pg_search--<prev>--<version>.sql`, assemble `docs/changelog/unreleased/*.mdx` into `docs/changelog/<version>.mdx`, register the release in `docs/docs.json`, and remove the consumed fragments.
   - Bump `workspace.package.version` in `Cargo.toml` and synchronize `Cargo.lock`.
   - Commit and push the release commit to `main`.
   - Create the Git tag `v<version>`, triggering downstream packaging and publishing workflows.
   - For minor releases (`x.y.0`), automatically create the stable branch `x.y.x` pointing at the release commit.

### Patch Releases

Fixes intended for a stable release are labeled with `cherry-pick/<branch>` (e.g. `cherry-pick/0.25.x`) on `main` and automatically backported via `.github/workflows/cherry-pick.yml` upon merge into the stable branch.

To publish a patch release from a stable branch:

1. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml).
2. Click **Run workflow**, select the stable branch (e.g. `0.25.x`) as the release branch, and provide the patch `version` (e.g. `0.25.5`).
3. The workflow will automatically:
   - Assemble the unreleased fragments present on the stable branch into `pg_search--<prev>--<version>.sql` and `docs/changelog/<version>.mdx`.
   - Bump `Cargo.toml` and synchronize `Cargo.lock` on the stable branch.
   - Commit, tag `v<version>`, and publish the GitHub release.
   - **Sync to `main`:** Check out `main`, copy the assembled SQL script and changelog page, update `docs/docs.json`, delete the consumed fragments from `main`, and push the sync commit to `main`.

## Post-Release Steps

1. **Verify** that the GitHub release and tag were created correctly and that all downstream packaging jobs completed successfully.
2. **Release** `paradedb/paradedb-enterprise` by following the instructions in the repository's RELEASE.md file.

That's it! Go for a walk, you deserve it.
