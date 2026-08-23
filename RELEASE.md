# ParadeDB Release Process

We use a single branch, `main`, for our development. Features are built on separate branches and merged into `main` via pull requests.

All releases are **manually triggered** using the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) in the GitHub Actions UI.

## Release Types

Releases must always be triggered **from the branch being released** (e.g., `main` for a minor or beta release, or a hotfix branch for patches).

| Type          | Description                                                                                               |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| **Minor**     | Triggered from the `main` branch.                                                                         |
| **Patch**     | A patch bump off an existing tag (e.g., `v1.4.0 → v1.4.1`).                                               |
| **Beta (RC)** | Marked with `beta: true`. Produces a prerelease tag like `vX.Y.Z-rc.N`. Requires `-rc.N` in `Cargo.toml`. |

> **Note:** Minor and patch releases publish Docker images for all supported PostgreSQL major versions and prebuilt extension binaries for all supported platforms. Beta releases publish only the PostgreSQL 18 Docker image and the Debian 13 packages required to build it; the remaining prebuilt extension binaries are skipped.

## Workflow Inputs

| Input          | Type    | Default | Description                                                                                    |
| -------------- | ------- | ------- | ---------------------------------------------------------------------------------------------- |
| `version`      | string  | `""`    | Target release version in semver format (e.g., `1.2.3` or `1.2.3-rc.1` for beta releases).     |
| `beta`         | boolean | `false` | If `true`, creates a beta release (`vX.Y.Z-rc.N`) and marks it as a pre-release in GitHub.     |
| `confirmation` | boolean | `false` | **Required** Confirms that version bump, SQL upgrade script, docs, and changelog are complete. |

> **Note:** The `version` provided _must_ match the version in the `Cargo.toml` file on the branch being released and contain `-rc.X` in the case of a beta release. The workflow will not run unless `confirmation: true`.

## Release Preparation

Before triggering the workflow, create a **Release Preparation PR** against `main`. For a patch release from a stable branch such as `0.25.x`, merge the preparation PR into `main`, backport it to the stable branch, and trigger the release workflow from the stable branch.

- Update the `Cargo.toml` version:
  - `a.b.c-rc.d` for **beta** releases
  - `a.b.0` for **minor** releases
  - `a.b.c` for **patch** releases
- Run `cargo check` to refresh `Cargo.lock` with the new version.
- Add a `pg_search--<previous-version>--<upcoming-version>.sql` upgrade script.
- Update the version references in the deployment and upgrade docs and in `docs/docs.json`.
- Write a changelog entry and add it to `docs/docs.json`.
- Ensure `nix/pg_search.nix` has the correct `cargoHash`. The Nix CI workflow updates it automatically for branches in this repository when necessary.

See the [0.25.3 release preparation PR](https://github.com/paradedb/paradedb/pull/5982) for a recent example.

## Triggering a Release

### Minor & Beta

To publish a minor or beta release from `main`:

1. Create and merge the Release Preparation PR
2. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
3. Click **Run workflow**, select `main` as the release branch, and set your inputs
4. Click **Run workflow** to start the job, and monitor the progress of the various jobs under the [GitHub Actions UI](https://github.com/paradedb/paradedb/actions)

### Patch

To publish a patch for an older release:

1. **Branch off** the target tag (e.g. `git checkout -b 0.16.x <release-tag>`), if a stable branch does not already exist
2. Cherry-pick the fixes you need into the stable branch
3. Create and merge the Release Preparation PR against `main`, then backport it to the stable branch
4. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
5. Click **Run workflow**, select the stable branch as the release branch, and set your inputs

## Post-Release Steps

1. **Verify** that the GitHub release and tag were created correctly and that all jobs completed successfully.
2. **Release** `paradedb/paradedb-enterprise` by following the instructions in the repository's RELEASE.md file.
3. **Open a post-release PR** against the released branch to bump `Cargo.toml` to the next development version (e.g., `0.24.2` after releasing `0.24.1`), add the corresponding empty upgrade script, run `cargo check` to refresh `Cargo.lock`, and update the Nix `cargoHash`.

> [!IMPORTANT]
> **Do step 3 before merging any new schema work.** Until the branch is bumped, the
> current upgrade script targets the version you just released, so it's frozen —
> changes added to it never reach users already on that version. The
> **Check Released Migrations** CI guard enforces this.

That's it! Go for a walk, you deserve it.
