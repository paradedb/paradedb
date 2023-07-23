<p align="center">
  <a href="https://retake.mintlify.app"><img src="assets/retake.png" alt="Retake" width="250px"></a>
</p>

<p align="center">
    <b>Open Source Infrastructure for Vector Data Streams</b> <br />
    Data pipelines that synchronize vectors with their sources of truth <br />
</p>

<h3 align="center">
  <a href="https://docs.getretake.com">Documentation</a> &bull;
  <a href="https://getretake.com">Website</a>
</h3>

<p align="center">
<a href="https://github.com/getretake/retake/stargazers/" target="_blank">
    <img src="https://img.shields.io/github/stars/getretake/retake?style=social&label=Star&maxAge=2592000" alt="Test">
</a>
<a href="https://github.com/getretake/retake/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/getretake/retake?color=white" alt="Release">
</a>
<a href="https://github.com/getretake/retake/tree/main/LICENSE" target="_blank">
    <img src="https://img.shields.io/static/v1?label=license&message=ELv2&color=white" alt="License">
</a>
</p>

## Installation

Welcome! If you are not a contributor and just want to use Retake, please
proceed to the [main branch](https://github.com/getretake/retake/tree/main).

To install the Retake Python SDK:

```bash
pip install retake
```

Follow the [documentation](https://retake.mintlify.app) for usage instructions.

## Key Features

**:arrows_counterclockwise: Out-of-the-Box Data Sync**

Existing vector stores are siloes that require complex and sometimes brittle
mechanisms for data synchronization. Retake provides the missing connectors that
allow seamless data synchronization without the need for extensive configuration
or third-party tools.

**:rocket: True Real-Time Updates**

Retake's connectors achieve sub-10ms end-to-end data latency, excluding variable
model inference times.

**:link: Extensible Python SDK**

You can configure any source, sink, transformation, and embedding model as code.
Joining and filtering tables or adding metadata is easily done from Python
functions.

**:zap: Scalable and Efficient**

Built on top of Kafka, Retake is designed to handle large volumes of data and
high-throughput workloads.

**:globe_with_meridians: Deployable Anywhere**

You can run Retake anywhere, from your laptop to a distributed cloud system.

## Development

If you are a developer who wants to contribute to Retake, follow these instructions to run Retake locally.

### Python SDK

The Python SDK enables users to define and configure vector data pipelines and
is responsible for all batch ETL jobs. To develop and run the Python SDK
locally:

1. Install Poetry

```bash
curl -sSL https://install.python-poetry.org | python -
```

2. Install dependencies

```bash
poetry install
```

3. Build the SDK locally

```bash
poetry build
```

This command will build and install the `retake` SDK locally. You can now
`import retake` from a Python environment.

### Real-Time Server

Built on top of Kafka, the real-time server sits between source(s) and
sink(s). It is responsible for all real-time data streams.

1. Ensure that Docker and Docker Compose are installed.

2. Ensure that Poetry and dependencies are installed (see Python SDK
   instructions above).

3. Start the development server, which is composed of the Kafka broker, Kafka Connect
   and the schema registry. Docker Compose will expose a port for each of the
   services (see `docker-compose.yml` for details).

```bash
docker compose up
```

4. To connect to the development server, refer to the [documentation](https://docs.getretake.com/quickstart/real-time-update).

## Contributing

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

## Licensing

Retake is [Elastic License 2.0 licensed](LICENSE).
