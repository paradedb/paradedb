# ParadeDB Dockerfiles

ParadeDB's Dockerfiles are automatically generated from `Dockerfile.template`. To make a change to the files, modify `Dockerfile.template`, run `./generate-dockerfiles.sh <<current-version>>`, and commit the generated changes.

## Generated Images

There are three flavors of files generated:

- `paradedb`: The default ParadeDB Docker image, published to `paradedb/paradedb`. Includes Barman Cloud which is used in our CNPG deployments.
- `official`: The image for Docker Official Images which will be published to `paradedb` once approved by Docker. Does not include Barman.
- `antithesis`: The image used by Antithesis test runs. Includes `libvoidstar`, Antithesis' instrumentation library.

`paradedb` and `official` both install Debian artifacts published to GitHub Releases. `antithesis` installs a locally built `.deb` so that it can be run on a per-commit basis.

## Extension Image

`Dockerfile.extension` is **not** generated from the template — it is a standalone, hand-maintained Dockerfile. Unlike the runnable images above, it builds a non-runnable `FROM scratch` artifact (`paradedb/paradedb-extension`) containing only pg_search's shared library, control file, SQL scripts, and license. It is meant to be **mounted** into a Postgres container by operators that support extension images — e.g. CloudNativePG, following the [cloudnative-pg/postgres-extensions-containers](https://github.com/cloudnative-pg/postgres-extensions-containers) `<version>-<pg-major>-<distro>` tag convention. It requires PostgreSQL 18+ (which introduced `extension_control_path`) and is built and published at release time alongside the runnable images by `publish-paradedb-docker.yml`.

## Release Process

Because the Dockerfiles depend on the Debian artifacts, they are published after the latest `.deb`s are published to GitHub. The Dockerfiles themselves can't be updated until the latest `.deb`s exist. Because of this, once a release is triggered and the `.deb`s are published, new versions of the Dockerfiles are generated in CI using the latest version. These new versions are then tested and published, and PRs are automatically opened to commit the updated files back to the repo. The files must be committed so they can be referenced by the Docker Official Images manifest file in [docker-library/official-images](https://github.com/docker-library/official-images).
