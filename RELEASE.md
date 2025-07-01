# ParadeDB Release Process

We maintain a single branch, `dev`, which represents the canonical state of the codebase. All development happens on feature branches, which are merged into `dev` via pull requests. **All** releases are triggered through the GitHub Actions UI, under the **Publish GitHub Release** job.

Standard releases, which are either minor, patch or beta releases (release candidate) to the previous latest release deploy the state of `dev` at the time of triggering a release. The release workflow will create a Git tag, marked as a pre-release in the case of a beta release,

TODO:

All releases except for hotfix releases publish the current state of `dev`, while hotfix releases publish the state of

and **always** publish the current state of `dev`. The release workflow will create a Git tag (and optionally a prerelease) and then publish Docker images and extension binaries.

---

## Workflow Inputs

When you click **Run workflow** on **Publish GitHub Release**, you must supply:

| Input          | Type    | Default | Description                                                                                                                     |
| -------------- | ------- | ------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `minor`        | boolean | `false` | Bump the **minor** version (`X.Y → X.(Y+1).0`). Resets patch and RC to `0`/`1`.                                                 |
| `patch`        | boolean | `false` | Bump the **patch** version (`X.Y.Z → X.Y.(Z+1)`). Resets RC to `1`.                                                             |
| `beta`         | boolean | `true`  | Create an **RC prerelease** of the current X.Y.Z (`vX.Y.Z-rc.N`). Increments RC counter by `+1`.                                |
| `hotfix`       | boolean | `false` | Do a **hotfix** release. Must also supply `hotfix_tag` to base the bump on.                                                     |
| `hotfix_tag`   | string  | `""`    | Tag to base a hotfix on (e.g. `v1.4.0`). Required if `hotfix: true`.                                                            |
| `confirmation` | boolean | _none_  | **Required**—check to confirm you’ve already bumped `Cargo.toml`, written the SQL upgrade script, and updated docs & changelog. |
| `dry_run`      | boolean | `false` | _Temporary/testing only_: prefixes the tag with `dryrun-` and skips variable updates.                                           |

> **Only one** of `minor`, `patch`, `hotfix`, or **no bump**+`beta` may be selected per run. The job will refuse to start unless you check `confirmation: true`.

---

## Release Preparation

Before you trigger the workflow, open a **“prep” PR** against `main` that includes:

- **`Cargo.toml` bump** to the target semver:
  - **Beta**: `a.b.c-rc.d` (the `-rc` suffix is required for prereleases).
  - **Stable** (minor/patch/hotfix): `a.b.c`.
- Run `cargo build` to regenerate `Cargo.lock`.
- **Stable** releases only:
  - Update version references in docs.
  - Add a changelog entry and update `docs/docs.json`.
- Create `pg_search--<old-version>--<new-version>.sql` upgrade script.

See [example prep PR](https://github.com/paradedb/paradedb/pull/2720).

---

## Triggering a Release

1. Go to **Actions → Publish GitHub Release**.
2. Click **Run workflow**, fill in the inputs:
   - Choose exactly one bump strategy:
     - **No bump + beta** (defaults) → prerelease of current version.
     - `patch: true` → patch bump.
     - `minor: true` → minor bump.
     - `hotfix: true` + `hotfix_tag: vX.Y.Z` → hotfix bump.
   - Leave `beta` on for prereleases, turn off for full releases.
   - Check **confirmation** (required).
   - (Optional) `dry_run: true` to test end-to-end without touching your real variables.
3. Hit **Run workflow**.

The workflow will:

1. Read `VERSION_MAJOR`, `VERSION_MINOR`, `VERSION_PATCH`, `VERSION_RC` from your Actions variables.
2. Compute the new tag:
   - **Beta** (no bump flags): `vX.Y.Z-rc.N`, bump `RC → RC+1`.
   - **Patch**: `vX.Y.(Z+1)`, bump `PATCH → PATCH+1`, reset `RC → 1`.
   - **Minor**: `vX.(Y+1).0`, bump `MINOR → MINOR+1`, reset `PATCH → 0` & `RC → 1`.
   - **Hotfix**: from `hotfix_tag` (e.g. `v1.4.0`), increment its patch → `v1.4.1`, bump `PATCH`, reset `RC → 1`.
   - If `dry_run`, the tag is prefixed `dryrun-` and variable updates are skipped.
3. Create the GitHub Release on branch `dev` with that tag, marking it prerelease if `beta: true`.
4. Unless in `dry_run` mode, PATCH your Actions variables (`VERSION_MINOR`, `VERSION_PATCH`, `VERSION_RC`) to their new values.

---

## Post-Release

After the workflow completes:

1. Verify the GitHub Release (and tag) is correct.
2. Open a **post-release PR** against `main` to bump `Cargo.toml` to the _next_ development version (e.g. `1.2.0-dev.0`).
3. Merge that PR so `main` always tracks the next unreleased state.

## Patch Releases

To publish a patch for an older version, first branch off the desired release's tag:

```bash
git checkout -b hotfix/1.4.x v1.4.0
```

Then, cherry-pick only the fixes you need into that branch.

Bump `Cargo.toml` to the desired version in that branch.

That's it!
