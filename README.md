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

## Installation

Welcome! If you are not a contributor and just want to use Retake, please
proceed to the [main branch](https://github.com/getretake/retake/tree/main).

To install the Retake Python SDK:

```
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
services (see `docker-compose.yaml` for details).

```
docker compose up
```

4. Install the `retake-cli`

```bash
cd retake_cli && poetry install
```

5. Configure a source and connector. 

```bash
poetry run retake-cli init
```

You will be propmpted for connection details, as well as the database schema and database
table to connect. **Keep in mind that the table has to exist and have at least
one record in order for the source connector to correctly create a topic.**

## Deployment

### Prerequisites

- The real-time server should be deployed on an instance with at least 4GB memory.
- The instance must expose an external port for Kafka. See
`docker-compose.yaml` file for the recommended configuration.

### Instructions 

1. Run the deploy script. It will use the `main` branch by default, but you
   can configure this with the `--branch` option.

```bash
curl https://raw.githubusercontent.com/getretake/retake/main/deploy.sh | bash
```

2. Once the docker compose stack is ready, run the `init` command with the cli.

```bash
retake-cli init
```

You will be prompted for connection details on the source and sink. **Ensure the
table exists and has at least 1 record so the topic is created correctly.**

### Usage

After the `init` command is done, the realtime server can be integrated with the
SDK by creating a `RealtimeServer` object:

```python
my_rt_server = RealtimeServer(host="0.0.0.0")
```

and you can start the realtime worker with:

```python
pipeline.pipe_real_time()
```

The worker will start listening for changes on the table and process the stream
with the models and transforms you define on the pipeline.

## Contributing

For more information on how to contribute, please see our
[Contributing Guide](CONTRIBUTING.md).

## Licensing

Retake is [Apache 2.0 licensed](LICENSE).
