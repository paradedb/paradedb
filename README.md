<p align="center">
  <img src="assets/retake.svg" alt="Retake" width="125px"></a>
</p>

<h1 align="center">
    <b>Retake</b>
</h1>

<p align="center">
    <b>Real-Time Universal Search Infra for Developers</b> <br />
</p>

<h3 align="center">
  <a href="https://docs.getretake.com">Documentation</a> &bull;
  <a href="https://getretake.com">Website</a>
</h3>

<p align="center">
<a href="https://github.com/getretake/retake/stargazers/" target="_blank">
    <img src="https://img.shields.io/github/stars/getretake/retake?style=social&label=Star&maxAge=60" alt="Stars">
</a>
<a href="https://github.com/getretake/retake/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/getretake/retake?color=white" alt="Release">
</a>
<a href="https://github.com/getretake/retake/tree/main/LICENSE" target="_blank">
    <img src="https://img.shields.io/static/v1?label=license&message=ELv2&color=white" alt="License">
</a>
</p>

Retake is the fastest way to implement universal search in your app.

Built to stay in sync with fast-changing databases. Retake abstracts away the complexity of search by acting as a search
layer around any Postgres database and providing simple search SDKs that snap into any Python or Typescript application. You don't need to worry about managing separate vector stores and text search engines, uploading and embedding documents, or reindexing data. Just write search queries and let Retake handle the rest.

To get started, simply start the Retake engine

```bash
docker compose up
```

By default, this will start the Retake engine at `http://localhost:8000` with API key `retake-test-key`.

## Usage

### Python

Install the SDK

```bash
pip install retakesearch
```

The core API is just a few functions.

```python
from retakesearch import Client, Index, Database, Table, Search

client = Client(api_key="retake-test-key", url="http://localhost:8000")

database = Database(
    host-"***",
    user="***",
    password="***",
    port=5432
)

table = Table(
    name="table_name",
    primary_key="primary_key_column",
    columns=["column1"] # These are the columns you wish to search
)

index = client.create_index("my_index")
index.add_source(database, table)

query = Search().neuralQuery("my query", ["column1"])
response = index.search(query)

print(response)
```

## Key Features

:arrows_counterclockwise: **Always in Sync**

Retake leverages Kafka to integrate directly with Postgres. As data changes or new data arrives,
Retake ensures that the indexed data is kept in sync.

:brain: **Intelligent Vector Cache**

Whenever data is changed in Postgres, Retake also updates the embedding/vector representation of that data behind the scenes. Vectors are automatically cached for lightning-fast query results with semantic understanding.

:rocket: **Low-Code SDK**

Retake provides intuitive search SDKs that drop into any Python application (other languages coming soon). The core API is just a few functions.

:zap: **Open/ElasticSearch DSL Compatible**

Retake’s query interface is built on top of the the high-level OpenSearch Python client, enabling developers to query with the full expressiveness of the OpenSearch DSL (domain-specific language).

:globe_with_meridians: **Deployable Anywhere**

Retake is deployable anywhere, from a laptop to a distributed cloud system.

## How Retake Works

A detailed overview of Retake's architecture can be found in our [documentation](https://docs.getretake.com/architecture).

## Contributing

For more information on how to contribute, please see our [Contributing Guide](CONTRIBUTING.md).

## License

Retake is [Elastic License 2.0 licensed](LICENSE).
