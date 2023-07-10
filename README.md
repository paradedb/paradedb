<p align="center">
  <a href="https://retake.mintlify.app"><img src="assets/retake.png" alt="Retake" width="250px"></a>
</p>

<p align="center">
    <b>Open Source Infrastructure for Vector Data Streams</b> <br />
    Retake provides data pipelines that sync vectors with their sources of truth <br />
</p>

<h3 align="center">
  <a href="https://docs.getretake.com">Documentation</a> &bull;
  <a href="https://getretake.com">Website</a>
</h3>

## Installation

Welcome! If you are not a contributor and just want to use Retake, please proceed to the [stable version](https://github.com/retake-earth/retake/tree/main).

To install the Retake Python SDK:

```
pip install retake
```

Follow the [documentation](https://retake.mintlify.app) for usage instructions.

## Development

### Python SDK

To develop and run the Python SDK locally, follow these steps:

1. Install Poetry

```
curl -sSL https://install.python-poetry.org | python -
```

2. Install dependencies

```
poetry install
```

3. Build the SDK locally

```
poetry build
```

This command will build and install the `retake` SDK locally. You can now `import retake` from a Python environment.

## Key Features

**:arrows_counterclockwise:  Out-of-the-Box Data Sync**

Existing vector stores are siloes that require complex and sometimes brittle mechanisms for data synchronization.
Retake provides the missing connectors that allow seamless data synchronization without the need for extensive
configuration or third-party tools.

**:rocket:  True Real-Time Updates**

Retake's connectors achieve sub-10ms end-to-end data latency, excluding variable model inference times.

**:link:  Extensible Python SDK**

You can configure any source, sink, transformation, and embedding model as code. Joining and filtering tables
or adding metadata is easily done from Python functions.

**:zap:  Scalable and Efficient**

Built on top of Kafka, Retake is designed to handle large volumes of data and high-throughput workloads.

**:globe_with_meridians:  Deployable Anywhere**

You can run Retake anywhere, from your laptop to a distributed cloud system.

## Contributing
For more information on how to contribute, please see our [Contributing Guide](CONTRIBUTING.md).

## Licensing
Retake is [Apache 2.0 licensed](LICENSE).
