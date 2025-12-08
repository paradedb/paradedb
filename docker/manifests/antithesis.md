# Antithesis in K8s

Step by step on getting Antithesis working in Kubernetes, with our CNPG deployment.

1- Folders

```bash
cd docker
mkdir antithesis
cd antithesis
```

2- Add CNPG Helm Repo

First, add the dependency on CNPG

```bash
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo update
```

Then, add our own repo

```bash
helm repo add paradedb https://paradedb.github.io/charts
helm repo update
```

3- Create values.yaml

```bash
touch values.yaml
```

Create file with

```yaml
type: paradedb
mode: standalone

cluster:
  instances: 1
  storage:
    size: 256Mi
```

4- Manifests

```bash
mkdir -p manifests
helm template paradedb paradedb/paradedb -f values.yaml > manifests/paradedb.yaml
```
