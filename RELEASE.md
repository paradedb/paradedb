# ParadeDB Release Process

We maintain a single branch, `main`, which represents the canonical state of the codebase. All development is done on feature branches, which are merged into `main` via pull requests. Releases are triggered manually through the GitHub Actions UI and **always** release the current state of `main`. This creates a Git tag for the version in `Cargo.toml`, which in turn publishes the Docker image and extension binaries.

## Release Preparation

Before triggering a release, create a "preparation" PR that includes:

- Bumping `Cargo.toml` to the target semver version.
  - For beta releases, use the format `a.b.c-rc.d`. The `-rc` format is **required** for beta release tooling to function correctly.
- Running `cargo build` to regenerate `Cargo.lock`.
- For stable releases, updating all version references in documentation.
- For stable releases, adding a changelog entry and updating `docs/docs.json`.
- Creating a `pg_search--<current-version>--<next-version>.sql` upgrade script.

See [this example preparation PR](https://github.com/paradedb/paradedb/pull/2720).

## Triggering a Release

To publish a release, go to the [Publish GitHub Release GitHub Actions UI](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) and click **Run workflow**. You'll be prompted to specify whether it's a beta release. Releases are marked as beta by default to prevent accidental stable releases.

## Post-Release

After the release is complete, create a new PR against `main` to bump the `Cargo.toml` version to the next development version, which represents the current unreleased state.

## Patch Releases

To publish a patch for an older version, first branch off the desired release's tag:

```bash
git checkout -b hotfix/1.4.x v1.4.0
```

Then, cherry-pick only the fixes you need into that branch.

Bump `Cargo.toml` to the desired version in that branch.

Merge the branch back into `main` using a merge commit (to preserve patch history).

Tag the merge commit with the new version and push it.

That's it!
