# ParadeDB Release Process

We use a single branch, `main`, for our development. Features are built on separate branches and merged into `main` via pull requests.

ParadeDB uses a **fragment-based workflow** for release artifacts:

- PRs that introduce user-facing changes add a changelog fragment to `docs/changelog/unreleased/<PR>.<category>.mdx`.
- PRs that modify extension DDL/schema add a SQL migration fragment to `pg_search/sql/unreleased/<PR>.<description>.sql`.

At release time, the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) automatically assembles these fragments into the versioned SQL upgrade script and changelog page.

## Enterprise: Community Sync

Commits from `paradedb/paradedb` are automatically synced to `paradedb/paradedb-enterprise` via GitHub Actions.

If conflicts occur during automated sync, you'll be notified via Slack with instructions on how to resolve them manually.

The sync workflow:

- Automatically applies all community commits
- Works entirely on patch branches created from `origin/main` - your local `main` is never modified
- Pushes patch branches to `origin/main` after CI validation passes
- Stops on conflicts and requires manual resolution

## Release Types

| Type          | Description                                                                                                                        |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Minor**     | Triggered from the `main` branch. Publishes `x.y.0` and creates stable branch `x.y.x`.                                             |
| **Patch**     | Triggered from a stable branch (e.g., `0.25.x`). Publishes `x.y.z` and syncs artifacts to `main`.                                  |
| **Beta (RC)** | Marked with `beta: true`. Can be triggered from any branch/commit. Produces a tag like `vX.Y.Z-rc.N` without modifying the branch. |

> **Note:** Minor and patch releases publish Docker images for all supported PostgreSQL major versions and prebuilt extension binaries for all supported platforms. Beta releases publish only the PostgreSQL 18 Docker image and the Debian 13 packages required to build it; the remaining prebuilt extension binaries are skipped.

## Workflow Inputs

| Input     | Type    | Default | Description                                                                                      |
| --------- | ------- | ------- | ------------------------------------------------------------------------------------------------ |
| `version` | string  | `""`    | Target release version in semver format (e.g., `1.2.3` or `1.2.3-rc.1` for beta releases).       |
| `beta`    | boolean | `false` | If `true`, creates a beta release (e.g., `vX.Y.Z-rc.N`) and marks it as a pre-release in GitHub. |

## Triggering a Release

### Minor Releases

To publish a minor release from `main`:

1. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
2. Click **Run workflow**, select `main` as the release branch, and provide the target `version` (e.g. `0.26.0`).
3. Click **Run workflow** to start the job.
4. The workflow will automatically:
   - Run `scripts/release.sh all` to assemble `pg_search/sql/unreleased/*.sql` into `pg_search--<prev>--<version>.sql`, assemble `docs/changelog/unreleased/*.mdx` into `docs/changelog/<version>.mdx`, register the release in `docs/docs.json`, and remove the consumed fragments.
   - Bump `workspace.package.version` in `Cargo.toml` and synchronize `Cargo.lock`.
   - Open a manual approval issue for `@pg_search-maintainers` displaying the release details and rendered changelog.
   - Upon approval (by commenting `approved` on the issue), commit and push the release commit to `main`.
   - Create the Git tag `v<version>`, triggering downstream packaging and publishing workflows.
   - Create the stable branch `x.y.x` pointing at the release commit.

### Beta (RC) Releases

To publish a beta release from any branch or commit:

1. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml)
2. Click **Run workflow**, select the desired branch or commit, provide the `version` (e.g. `0.26.0-rc.0` or `0.25.6-rc.1`), and check `beta: true`.
3. Click **Run workflow** to start the job.
4. The workflow will automatically:
   - Assemble `pg_search/sql/unreleased/*.sql` into `pg_search--<prev>--<version>.sql` while preserving the unreleased fragments.
   - Skip changelog and docs generation.
   - Create and push the Git tag `v<version>` pointing directly to the release commit, without pushing commits to the release branch.

### Patch Releases

Fixes intended for a stable release are labeled with `cherry-pick/<branch>` (e.g. `cherry-pick/0.25.x`) on `main` and automatically backported via `.github/workflows/cherry-pick.yml` upon merge into the stable branch.

To publish a patch release from a stable branch:

1. Go to [Actions → Publish GitHub Release](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml).
2. Click **Run workflow**, select the stable branch (e.g. `0.25.x`) as the release branch, and provide the patch `version` (e.g. `0.25.5`).
3. The workflow will automatically:
   - Assemble the unreleased fragments present on the stable branch into `pg_search--<prev>--<version>.sql` and `docs/changelog/<version>.mdx`.
   - Bump `Cargo.toml` and synchronize `Cargo.lock` on the stable branch.
   - Open a manual approval issue for `@pg_search-maintainers` displaying the release details and rendered changelog.
   - Upon approval (by commenting `approved` on the issue), commit, tag `v<version>`, and publish the GitHub release.
   - **Sync to `main`:** Check out `main`, copy the assembled SQL script and changelog page, update `docs/docs.json`, delete the consumed fragments from `main`, and push the sync commit to `main`.

## Enterprise Release

**Releases are always performed first on `paradedb/paradedb`: see above.**

After executing the community release, executing an enterprise release involves:

### Minor Releases (Enterprise)

For a minor release (e.g. `0.22.0`):

1. Manually trigger a rebase or community sync such that the community release commit (e.g. `chore: Prepare 0.22.0.`) has been synced to `origin/main`.
2. Trigger a release on `main` using the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml).
3. The workflow will automatically:
   - Open a manual approval issue for `@pg_search-maintainers`.
   - Upon approval (by commenting `approved` on the issue), tag `v<version>`, create the stable branch (e.g. `0.22.x`), and publish the GitHub release.

### Beta (RC) Releases (Enterprise)

For a beta release (e.g. `0.22.0-rc.0`):

Trigger the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml) on the desired branch or commit with `beta: true`, matching the community process.

### Patch Releases (Enterprise)

For a patch release (e.g. `0.22.2`), stable branches should already exist in both `paradedb/paradedb` and `paradedb/paradedb-enterprise` with the same name (e.g. `0.22.x`).

After executing the community release, you should sync all new commits on the community stable branch to the enterprise stable branch:

1. Create a sync branch from the enterprise stable branch:
   - Something like: `git checkout -b sync-0.22.2 origin/0.22.x` (where `origin` is your enterprise remote)
2. Cherry-pick all new commits from the community stable branch into your sync branch:
   - Something like `git cherry-pick <prev_commit>...upstream/0.22.x` (where `upstream` is your community remote)
3. Open a PR for your sync branch on enterprise, targeted at the stable branch, and get it reviewed.
4. Land the PR with `Rebase and Merge`, then trigger a release on the stable branch using the [**Publish GitHub Release** workflow](https://github.com/paradedb/paradedb/actions/workflows/publish-github-release.yml).

## Post-Release

Verify that the GitHub release and tag were created correctly and that all downstream packaging jobs completed successfully.

That's it! Go for a walk, you deserve it.
