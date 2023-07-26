<p align="center">
  <img src="assets/retake.svg" alt="Retake" width="125px"></a>
</p>

<h1 align="center">
    <b>Retake</b>
</h1>

<p align="center">
    <b>Real-Time Neural Search for Developers</b> <br />
</p>

Retake is real-time keyword + semantic neural search infrastructure for developers, built to stay in sync with fast-changing databases. Retake wraps around any Postgres database and provides simple search SDKs that snap into any Python or Typescript application. You don't need to worry about managing separate vector stores and text search engines, uploading and embedding documents, or reindexing data. Just write search queries and let Retake handle the rest.

To get started, simply start the Retake engine

```
docker compose up
```

By default, this will start the Retake engine at `http://localhost:8000` with API key `retake-test-key`.

# Usage

## Python

Install the SDK

```
pip install retakesearch
```

The core API is just two functions

```
from retakesearch import Client, Search, Database, Table

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
    columns: ["column1"] # These are the columns you wish to search
    neural_columns=["column1"] # These are the columns you wish to enable neural search over
)

# Index your table
# This only needs to be done once
client.index(database, table)

# Search your table
query = Search().neuralQuery("my_query", ["column1])
response = client.search("table_name", query)

print(response)
```

## Node.js

Install the SDK

```
npm install retake-search
```

The core API is just two functions

```
const { Client, Search, helpers } = require("retake-search");

# See docker-compose.yml for Default API key and URL 
const client = new Client({
  apiKey: "retake-test-key",
  url: "http://localhost:8000",
})

# Index your table
# This only needs to be done once
client.index({
  database: {
    host: ***,
    user: ***,
    password: ***,
    port: ***
  },
  table: {
    name: "table_name",
    primaryKey: "primary_key_column",
    columns: ["column1"] # These are the columns you wish to search
  }
})

# Search your table
const query = Search().query(helpers.matchQuery("column1", "my_query"))
const response = client.search("table_name", query)

console.log(response)
```

## React

Retake provides React hooks for better ergonomics. First, wrap your application in `SearchProvider`:

```
import { SearchProvider } from "retake-search"
import App from ./app.tsx

export default () => {
  return (
    <SearchProvider apiKey="retake-test-key" url="http://localhost:8000">
      <App/>
    </SearchProvider>
  )
}

```

Any component inside `SearchProvider` can invoke `useSearch`:

```
import { helpers, request, useSearch } from "retake-search";

const MyComponent = () => {
  const table = "table_name"
  const columns = ["column1", "column2"]

  const query = request.query(multiMatchQuery(columns, userInput)).toJSON()

  const { data, error } = useSearch({ table, query })

  return (
    <div>
      // Render data here
    </div>
  )
}
```

# Key Features

- Always in Sync
- Low-Code SDK
- Open/ElasticSearch DSL Compatible
- Hybrid Keyword + Semantic Search