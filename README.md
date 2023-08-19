<p align="center">
  <img src="assets/retake.svg" alt="Retake" width="125px"></a>
</p>

<h1 align="center">
    <b>Retake</b>
</h1>

<p align="center">
    <b>The Real-Time Database for AI Data</b> <br />
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
    <img src="https://img.shields.io/static/v1?label=license&message=Apache-2.0&color=white" alt="License">
</a>
</p>

Retake is a real-time database for AI data. By integratating your Postgres data with embedding models,
Retake enables real-time search over your database.

Documents in Retake are stored in **real-time indexes**. These indexes stay in sync with Postgres and
can be queried with keyword search, vector search, or hybrid search — a combination of the two.

To get started, run our Docker Compose file:

```bash
git clone git@github.com:getretake/retake.git
cd retake/docker
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
    host="***",
    user="***",
    password="***",
    port=5432
    dbname="***"
)

columns = ["column1"]
table = Table(
    name="table_name",
    columns=columns
)

index = client.create_index("my_index")
# Note: The table must have a primary key
index.add_source(database, table)
index.vectorize({ fieldNames: columns })

# Keyword (BM25) search
query = Search().query("match", column1="my query")
response = index.search(query)

# Semantic (vector-based) search
query = Search().with_semantic("my_query", columns)
response = index.search(query)

# Neural (keyword + semantic) search
query = Search().with_neural("my_query", columns)
response = index.search(query)

print(response)
```

### Typescript

Install the SDK

```
npm install retake-search
```

The core API is just a few functions.

```typescript
import { Client, Database, Table, Search } from "retake-search"
import { withSemantic, withNeural, matchQuery } from "retake-search/helpers"

const client = new Client("retake-test-key", "http://localhost:8000")

// Replace with your database credentials
const columns = ["column_to_search"]
const database = new Database({
  host: "***",
  user: "***",
  password: "***",
  dbName: "***",
  port: 5432,
})
const table = new Table({
  table: "table_name",
  columns: columns,
})

const index = await client.createIndex("table_name")

// Note: The table must have a primary key
await index.addSource(database, table)
await index.vectorize({ fieldNames: columns })

// Keyword (BM25) search
const bm25Query = Search().query(matchQuery("column_to_search", "my query"))
index.search(bm25Query)

// Semantic (vector-based) search
const semanticQuery = Search().query(withSemantic("my query", columns))
index.search(semanticQuery)

// Neural (keyword + semantic) search
const neuralQuery = Search().query(withNeural("my query", columns))
index.search(neuralQuery)
```

## Key Features

:arrows_counterclockwise: **Always in Sync**

Retake leverages logical-replication-based Change-Data-Capture (CDC) to integrate directly with Postgres. As data changes or new data arrives, Retake ensures that the indexed data is kept in sync.

:brain: **Intelligent Vector Cache**

Whenever data is changed in Postgres, Retake also updates the embedding/vector representation of that data behind the scenes. Vectors are automatically cached for lightning-fast query results with semantic understanding.

:rocket: **Low-Code SDK**

Retake provides intuitive search SDKs that drop into any Python or Typescript application (other languages coming soon). The core API is just a few functions.

:zap: **Open/ElasticSearch DSL Compatible**

Retake enables developers to query with the full expressiveness of the OpenSearch DSL (domain-specific language).

:globe_with_meridians: **Deployable Anywhere**

Retake is deployable anywhere, from a laptop to a distributed cloud system.

## How Retake Works

A detailed overview of Retake's architecture can be found in our [documentation](https://docs.getretake.com/architecture).

## Contributing

For more information on how to contribute, please see our [Contributing Guide](CONTRIBUTING.md).

## License

Retake is licensed under the [Apache-2.0 License](LICENSE).
