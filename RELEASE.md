# ParadeDB Release Process

We use a single `dev` branch for our development. All feature work happens on feature branches and is merged into `dev` via pull requests. Every release—minor, patch, beta (release candidate), or hotfix—is triggered manually via the **Publish GitHub Release** workflow in the GitHub Actions UI.

---

## Release Types

- **Minor Release** (`minor: true`):
  Bumps the minor version (`X.Y → X.(Y+1).0`), resets patch to `0` and RC to `1`. Deploys the current `dev` branch.
- **Patch Release** (`patch: true`):
  Bumps the patch version (`X.Y.Z → X.Y.(Z+1)`), resets RC to `1`. Deploys the current `dev` branch.
- **Beta (RC) Release** (`beta: true`, no bump flags):
  Creates a prerelease tag (`vX.Y.Z-rc.N`), increments the RC counter. Deploys the current `dev` branch as a release candidate and requires the `Cargo.toml` version to contain `-rc`.
- **Hotfix Release** (`hotfix: true`, `hotfix_tag`, `hotfix_branch`):
  Creates a patch bump off an existing tag (e.g. `v1.4.0 → v1.4.1`) using the specified `hotfix_branch`.
- **Dry Run** (`dry_run: true`):
  Prefixes the tag with `dryrun-` and skips variable updates, allowing safe end-to-end testing.

> **Note:** The Minor, Patch and Hotfix releases publish Docker images for all supported PostgreSQL major versions and prebuilt extension binaries for all supported platforms. The Beta release only publishes a Docker image for the default PostgreSQL major version and does not release prebuilt extension binaries.

---

## Workflow Inputs

| Input           | Type    | Default | Description                                                                                             |
| --------------- | ------- | ------- | ------------------------------------------------------------------------------------------------------- |
| `minor`         | boolean | `false` | Bump minor version (`X.Y → X.(Y+1).0`).                                                                 |
| `patch`         | boolean | `false` | Bump patch version (`X.Y.Z → X.Y.(Z+1)`).                                                               |
| `beta`          | boolean | `false` | Create an RC prerelease of the current version (`vX.Y.Z-rc.N`).                                         |
| `hotfix`        | boolean | `false` | Publish a hotfix. Requires `hotfix_tag` and `hotfix_branch`.                                            |
| `hotfix_tag`    | string  | `""`    | Base tag for hotfix (e.g. `v1.4.0`).                                                                    |
| `hotfix_branch` | string  | `""`    | Branch to release for the hotfix (branched off `hotfix_tag`).                                           |
| `confirmation`  | boolean | `false` | **Required**—confirm you’ve bumped `Cargo.toml`, written the SQL upgrade, and updated docs & changelog. |
| `dry_run`       | boolean | `false` | _Testing only_—prefixes tag with `dryrun-`, skips variable updates.                                     |

> **Note:** Only one of `minor`, `patch`, or `hotfix` may be `true`. A beta release requires no bump flags. The workflow will not run unless `confirmation: true`.

---

## Release Preparation

Before running the workflow:

1. **Create a prep PR** against `dev` that:
   - Updates `Cargo.toml` to the target semver:
     - `a.b.c-rc.d` for **beta**.
     - `a.b.c` for **stable** (minor/patch/hotfix).
   - Runs `cargo build` to refresh `Cargo.lock`.
   - (Stable only) Updates version references in docs, adds a changelog entry, and updates `docs/docs.json`.
   - Adds `pg_search--<old-version>--<new-version>.sql` upgrade script.

See [example prep PR](https://github.com/paradedb/paradedb/pull/2720).

---

## Triggering a Release

1. Go to **Actions → Publish GitHub Release**.
2. Click **Run workflow** and set your inputs.
3. Click **Run workflow** to start the job.

---

## Post-Release Steps

1. **Verify** the GitHub Release and tag.
2. **Open a post-release PR** against `dev` to bump `Cargo.toml` to the next development version (e.g. `1.2.0-rc.1`).
3. **Merge** that PR so `dev` reflects ongoing work.
   4 **Release** `paradedb/paradedb-enterprise`, `paradedb/charts` and `paradedb/terraform-paradedb-byoc`. More context to come here as we automate more of the release flow.

---

## Hotfix Releases

To publish a patch for an older release:

1. **Branch off** the target tag:

```bash
  git checkout -b hotfix/<version>.x <release-tag>
  # e.g. git checkout -b hotfix/0.15.15 v0.15.15
```

2. Cherry-pick the fixes you need into your hotfix branch.

3. Bump `Cargo.toml` to the new patch version in that branch and refresh the `Cargo.lock`.

4. Run the Publish GitHub Release workflow with:

- hotfix: `true`
- hotfix_tag: `<the-tag-from-which-you-branched>`
- hotfix_branch: `<the-name-of-your-hotfix-branch>`
