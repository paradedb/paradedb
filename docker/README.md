# ParadeDB Dockerfiles

The runnable ParadeDB Dockerfiles are generated from `Dockerfile.template`. To change one, modify the template, run `./generate-dockerfiles.sh <current-version>`, and commit the generated files. The specialized Dockerfiles documented below are maintained by hand.

## Generated Images

There are three flavors of files generated:

- `paradedb`: The default ParadeDB Docker image, published to `paradedb/paradedb`. Includes Barman Cloud which is used in our CNPG deployments.
- `official`: The image for Docker Official Images which will be published to `paradedb` once approved by Docker. Includes only `pg_search` and its required `pgvector` dependency, initialized in `template1`, `paradedb`, and `POSTGRES_DB`.
- `antithesis`: The image used by Antithesis test runs. Its `pg_search` is built with [Antithesis coverage instrumentation](https://antithesis.com/docs/reference/sdk/rust/instrumentation); `libvoidstar` is injected at runtime rather than baked into the image.

`paradedb` and `official` both install Debian artifacts published to GitHub Releases. `antithesis` installs a locally built `.deb` so that it can be run on a per-commit basis.

## Extension Image

`Dockerfile.extension` is **not** generated from the template — it is a standalone, hand-maintained Dockerfile. Unlike the runnable images above, it builds a non-runnable `FROM scratch` artifact (`paradedb/paradedb-extension`) that is meant to be **mounted** into a Postgres container by operators that support extension images — e.g. CloudNativePG, following the [cloudnative-pg/postgres-extensions-containers](https://github.com/cloudnative-pg/postgres-extensions-containers) `<version>-<pg-major>-<distro>` tag convention. It requires PostgreSQL 18+ (which introduced `extension_control_path`) and is built and published at release time alongside the runnable images by `publish-paradedb-docker.yml`.

The payload is laid out as `lib/` (pg_search.so), `share/extension/` (control file and SQL scripts), `licenses/`, and `system/` — the system libraries pg_search links that the Postgres base image lacks (`libopenblas`, `libgfortran`). Consumers must put `system/` on `LD_LIBRARY_PATH`, since `dynamic_library_path` finds the module but not the libraries it needs. Under CloudNativePG that is `ld_library_path: [system]` on the extension:

```yaml
postgresql:
  shared_preload_libraries:
    - pg_search
  extensions:
    - name: pg_search
      image:
        reference: paradedb/paradedb-extension:0.25.3-18-trixie
      ld_library_path:
        - system
    - name: pgvector
      image:
        reference: ghcr.io/cloudnative-pg/pgvector:0.8.6-18-trixie
```

`pg_search` requires the `vector` extension. The example mounts CloudNativePG's [pgvector extension image](https://github.com/cloudnative-pg/postgres-extensions-containers/tree/main/pgvector); alternatively, `vector` can be provided by the Postgres image itself.

## Manifests

`manifests/cnpg.yaml` is the CloudNativePG operator, rendered from the upstream Helm chart with default values and bundled into the Antithesis config images. To upgrade the operator, re-render it with the chart version whose `appVersion` is the CNPG release you want ([chart index](https://github.com/cloudnative-pg/charts)):

```bash
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update
helm template cnpg cnpg/cloudnative-pg --namespace cnpg-system --version <chart_version>
```

Prepend the `cnpg-system` `Namespace` (the chart does not create it) and keep the trailing comment-only documents at the end of the file. The current file is chart `0.29.0` / CNPG `1.30.0`.

## Release Process

Because the Dockerfiles depend on the Debian artifacts, they are published after the latest `.deb`s are published to GitHub. The Dockerfiles themselves can't be updated until the latest `.deb`s exist. Because of this, once a release is triggered and the `.deb`s are published, new versions of the Dockerfiles are generated in CI using the latest version. These new versions are then tested and published, and PRs are automatically opened to commit the updated files back to the repo. The files must be committed so they can be referenced by the Docker Official Images manifest file in [docker-library/official-images](https://github.com/docker-library/official-images).
