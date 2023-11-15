<h1 align="center">
  <img src="https://raw.githubusercontent.com/paradedb/paradedb/dev/docs/logo/readme.svg" alt="ParadeDB" width="368px"></a>
<br>
</h1>

<p align="center">
    <b>PostgreSQL for Search</b> <br />
</p>

<h3 align="center">
  <a href="https://paradedb.com">Website</a> &bull;
  <a href="https://docs.paradedb.com">Documentation</a> &bull;
  <a href="https://paradedb.com/blog">Blog</a> &bull;
  <a href="https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ">Community</a>
</h3>

---

# Helm Chart

This repository contains the Helm chart for deploying and managing ParadeDB on
Kubernetes.

## Prerequisites

- A Kubernetes cluster with at least v1.21
- [Helm](https://helm.sh/)
- [CloudNative Operator](https://cloudnative-pg.io/) installed on the cluster

## Usage

The steps below assume you have an accessible Kubernetes cluster.

### Install Helm

First, install Helm. You can do so using their installation script:

```bash
curl -fsSL -o get_helm.sh https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3
chmod 700 get_helm.sh
./get_helm.sh
```

See the [Helm docs](https://helm.sh/docs/intro/install/) for more information.

### Install CloudNative Operator

This chart does not include the Custom Resource Definitions (CRDs) from the
CloudNative Operator, and it doesn't explicitly depend on it due to Helm's
constraints with CRD management. As such, the operator itself is not bundled
within this chart.

To use this chart, you need to independently install the operator CRDs. You can
install the operator using the
[official helm chart](https://github.com/cloudnative-pg/charts).

```bash
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm upgrade --install cnpg \
  --namespace cnpg-system \
  --create-namespace \
  cnpg/cloudnative-pg
```

It is also possible to install using the manifest directly. See the operator
[installation documentation](https://cloudnative-pg.io/documentation/1.21/installation_upgrade/#installation-on-kubernetes)
for more information.

### Install ParadeDB Helm Chart

Once the operator is installed, add the ParadeDB repo to helm as follows:

    helm repo add paradedb https://paradedb.github.io/helm-charts

If you had already added this repo earlier, run `helm repo update` to retrieve
the latest versions of the packages. You can then run
`helm search repo paradedb` to see the charts.

To install the paradedb chart:

    helm install my-db paradedb/paradedb

To uninstall the chart:

    helm delete my-db

## Configuration

The ParadeDB Helm chart can be configured using the values.yaml file or by
specifying values on the command line during installation.

Check the
[values.yaml](https://github.com/paradedb/helm-charts/blob/main/charts/paradedb/values.yaml)
file for more information.

## Development

For local development, its recommended to use a local Kubernetes cluster like
[Minikube](https://minikube.sigs.k8s.io/docs/) or
[kind](https://kind.sigs.k8s.io/). Then install by doing the following:

1. Clone this repository:

```bash
git clone https://github.com/paradedb/helm-charts && cd charts
```

2. Change into the charts directory:

```bash
cd helm-charts/charts
```

3. Build dependencies:

```bash
helm dep up
```

4. Install the chart using Helm:

```bash
helm install paradedb paradedb --namespace paradedb --create-namespace
```

You are set!

## Contributing

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

## License

ParadeDB is licensed under the
[GNU Affero General Public License v3.0](LICENSE).
