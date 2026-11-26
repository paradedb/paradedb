# ParadeDB Dockerfiles

The runnable ParadeDB Dockerfiles are generated from `Dockerfile.template`. To change one, modify the template, run `./generate-dockerfiles.sh <current-version>`, and commit the generated files. The specialized Dockerfiles documented below are maintained by hand.

## Generated Images

There are two flavors of files generated:

- `paradedb`: The default ParadeDB Docker image, published to `paradedb/paradedb`. Includes Barman Cloud which is used in our CNPG deployments.
- `official`: The image for Docker Official Images which will be published to `paradedb` once approved by Docker. Includes only `pg_search` and its required `pgvector` dependency, initialized in `template1`, `paradedb`, and `POSTGRES_DB`.

## Source Image

`Dockerfile.source` compiles a selected source revision inside Docker and overlays it onto a released ParadeDB image for workload testing. Set `DST=true` to build the instrumented `dst` profile with split debug symbols in `/symbols`. DST runs currently use Antithesis, which injects `libvoidstar` at runtime. The build verifies coverage instrumentation and unresolved Rust symbols before shipping the image.

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

`manifests/cnpg.yaml` bundles the CloudNativePG operator into the Stressgres and property-test DST config images. It is based on the [upstream CloudNativePG 1.31.0-rc1 install manifest](https://github.com/cloudnative-pg/cloudnative-pg/releases/download/v1.31.0-rc1/cnpg-1.31.0-rc1.yaml).

To upgrade, download the install manifest for the target release and carry forward the Antithesis Deployment settings: two replicas with leader election enabled, `imagePullPolicy: IfNotPresent`, and `resources: {}`.

## Release Process

Because the Dockerfiles depend on the Debian artifacts, they are published after the latest `.deb`s are published to GitHub. The Dockerfiles themselves can't be updated until the latest `.deb`s exist. Because of this, once a release is triggered and the `.deb`s are published, new versions of the Dockerfiles are generated in CI using the latest version. These new versions are then tested and published, and PRs are automatically opened to commit the updated files back to the repo. The files must be committed so they can be referenced by the Docker Official Images manifest file in [docker-library/official-images](https://github.com/docker-library/official-images).
