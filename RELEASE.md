# ParadeDB Release Process

We use a single branch, `main`, for our development. Features are built on separate branches and merged into `main` via pull requests.

All releases are **manually triggered** using the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) in the GitHub Actions UI.

## Release Types

Releases must always be triggered **from the branch being released** (e.g., `main` for a minor or beta release, or a hotfix branch for patches).

| Type          | Description                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| **Minor**     | Triggered from the `dev` branch.                                                                        |
| **Patch**     | A patch bump off an existing tag (e.g., `v1.4.0 → v1.4.1`).                                             |
| **Beta (RC)** | Marked with `beta: true`. Produces a prerelease tag like `vX.Y.Z-rc.N`. Requires `-rc` in `Cargo.toml`. |

> **Note:** The Minor and Patch releases publish Docker images for all supported PostgreSQL major versions and prebuilt extension binaries for all supported platforms. The Beta release only publishes a Docker image for the default PostgreSQL major version and does not release prebuilt extension binaries.

## Workflow Inputs

| Input          | Type    | Default | Description                                                                                    |
| -------------- | ------- | ------- | ---------------------------------------------------------------------------------------------- |
| `version`      | string  | `""`    | Target release version in semver format (e.g., `1.2.3` or `1.2.3-rc.1` for beta releases).     |
| `beta`         | boolean | `false` | If `true`, creates a beta release (`vX.Y.Z-rc.N`) and marks it as a pre-release in GitHub.     |
| `confirmation` | boolean | `false` | **Required** Confirms that version bump, SQL upgrade script, docs, and changelog are complete. |

> **Note:** The `version` provided _must_ match that of the `Cargo.toml` of the branch being released file and contain `-rc.X` in the case of a beta release. The workflow will not run unless `confirmation: true`.

## Release Preparation

Before triggering the workflow, create a **Release Prepation PR** against `main`:

- Update the `Cargo.toml` version:
  - `a.b.c-rc.d` for **beta** releases
  - `a.b.0` for **minor** releases
- Run `cargo check` to refresh the `Cargo.lock` file with the new version
- Add a `pg_search--<previous-version>--<upcoming-version>` upgrade script
- (Minor only) Update the version references in the upgrade docs and in `docs/docs.json`
- (Minor and patch only) Write a changelog entry and add it to `docs/docs.json`

Here is an [example release preparation PR](https://github.com/paradedb/paradedb/pull/2770) for your reference.

## Triggering a Release

### Minor & Beta

To publish a minor or beta release for the current ongoing latest `main`:

1. Create and merge the Release Preparation PR
2. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
3. Click **Run workflow**, select `main` as the release branch, and set your inputs
4. Click **Run workflow** to start the job, and monitor the progress of the various jobs under the [GitHub Actions UI](https://github.com/paradedb/paradedb/actions)

### Patch

To publish a patch for an older release:

1. **Branch off** the target tag (e.g. `git checkout -b patch/<version>.x <release-tag>`)
2. Cherry-pick the fixes you need into your patch branch
3. Complete the Release Preparation PR work in your patch branch
4. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
5. Click **Run workflow**, select your patch branch as the release branch, and set your inputs

## Post-Release Steps

1. **Verify** that the GitHub Release and GitHub Tag properly created and that all jobs completed.
2. **Open a post-release PR** against `main` to bump `Cargo.toml` to the next development version (e.g. `0.20.0` or `0.20.0-rc.1`).
3. **Merge** that PR so `main` reflects ongoing work.
4. **Release** `paradedb/paradedb-enterprise`, `paradedb/charts` and `paradedb/terraform-paradedb-byoc`. More context to come here as we automate more of the release flow.

That's it! Go for a walk, you deserve it.
