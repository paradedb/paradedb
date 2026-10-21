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
helm template cnpg cnpg/cloudnative-pg --namespace cnpg-system --version <chart_version> --set replicaCount=2
```

Prepend the `cnpg-system` `Namespace` (the chart does not create it) and keep the trailing comment-only documents at the end of the file. The two operator replicas retain leader election so reconciliation can continue when one operator is unavailable. The current file is chart `0.29.1` / CNPG `1.30.1`.

## Antithesis Finding Suppressions

Antithesis configures finding suppressions on its side. The specification below documents a proposed rule to request from the Antithesis team, following its [suppression workflow](https://antithesis.com/docs/product/understanding_test_results/reports/findings/#disentangle-a-known-issue). Adding this documentation does not activate a suppression, and no activation is recorded here.

### CNPG Operator Leader Election Lost

CloudNativePG intentionally exits when it loses its Kubernetes leader-election lease. API connectivity loss can prevent lease renewal and trigger this safety behavior. The operator logs `problem running manager` with error `leader election lost` before exiting with code 1. See the upstream [controller-runtime leadership-loss handler](https://github.com/kubernetes-sigs/controller-runtime/blob/v0.25.1/pkg/manager/internal.go#L581-L591) and [CNPG manager error handling](https://github.com/cloudnative-pg/cloudnative-pg/blob/v1.30.1/internal/cmd/manager/controller/controller.go).

Request the following suppression, subject to Antithesis confirming support for correlating the exit with preceding logs:

| Field         | Match                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------- |
| Property      | `No unexpected container exits`                                                          |
| Namespace     | `cnpg-system`                                                                            |
| Pod           | Prefix `cnpg-cloudnative-pg-`                                                            |
| Container     | `manager`                                                                                |
| Exit code     | `1`                                                                                      |
| Preceding log | Same container instance logs `problem running manager` with error `leader election lost` |

Rationale: this specific exit is the operator's expected response to losing leadership. Keep other operator errors and all database container failures visible. Matching only the container name and exit code would also hide unrelated failures. If log correlation is unavailable, resolve the scope with Antithesis before enabling a broader rule.

Once Antithesis configures the rule, the report's **Suppressions** toggle controls whether suppressed findings are displayed. Keep recovery-liveness assertions enabled: an expected operator exit does not establish that connectivity recovered or another operator took leadership. In particular, connectivity failures during a reported fault-free window still need investigation. Preserve the operator's original exit codes and leader election behavior.

## Release Process

Because the Dockerfiles depend on the Debian artifacts, they are published after the latest `.deb`s are published to GitHub. The Dockerfiles themselves can't be updated until the latest `.deb`s exist. Because of this, once a release is triggered and the `.deb`s are published, new versions of the Dockerfiles are generated in CI using the latest version. These new versions are then tested and published, and PRs are automatically opened to commit the updated files back to the repo. The files must be committed so they can be referenced by the Docker Official Images manifest file in [docker-library/official-images](https://github.com/docker-library/official-images).
