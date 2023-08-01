import ky from "ky"
import helpers from "opensearch-js"

class Database {
  host: string
  user: string
  password: string
  port: number
  dbName: string

  constructor(
    host: string,
    user: string,
    password: string,
    port: number,
    dbName: string
  ) {
    this.host = host
    this.user = user
    this.password = password
    this.port = port
    this.dbName = dbName
  }
}

class Table {
  name: string
  primaryKey: string
  columns: string[]

  constructor(name: string, primaryKey: string, columns: string[]) {
    this.name = name
    this.primaryKey = primaryKey
    this.columns = columns
  }
}

class Index {
  indexName: string
  apiKey: string
  url: string
  headers: Record<string, string>

  constructor(indexName: string, apiKey: string, url: string) {
    this.indexName = indexName
    this.apiKey = apiKey
    this.url = url
    this.headers = {
      Authorization: `Bearer ${this.apiKey}`,
      "Content-Type": "application/json",
    }
  }

  async addSource(database: Database, table: Table) {
    const json = {
      index_name: this.indexName,
      source_host: database.host,
      source_user: database.user,
      source_password: database.password,
      source_port: database.port,
      source_dbname: database.dbName,
      source_relation: table.name,
      source_primary_key: table.primaryKey,
      source_columns: table.columns,
    }

    console.log(
      `Adding ${table.name} to index ${this.indexName}. This could take some time if the table is large...`
    )

    return await ky
      .post(`${this.url}/index/add_source`, {
        headers: this.headers,
        json: json,
      })
      .json()
  }

  async search(search: helpers.RequestBodySearch) {
    const json = {
      dsl: search.toJSON(),
      index_name: this.indexName,
    }

    return await ky
      .post(`${this.url}/index/search`, {
        headers: this.headers,
        json: json,
      })
      .json()
  }

  async upsert(
    documents: Record<string, any>[],
    ids: (string | number)[]
  ): Promise<any> {
    const json = { index_name: this.indexName, documents: documents, ids: ids }

    return await ky
      .post(`${this.url}/index/upsert`, {
        headers: this.headers,
        json: json,
      })
      .json()
  }

  async createField(fieldName: string, fieldType: string) {
    const json = {
      index_name: this.indexName,
      field_name: fieldName,
      field_type: fieldType,
    }

    return await ky
      .post(`${this.url}/index/field/create`, {
        headers: this.headers,
        json: json,
      })
      .json()
  }

  async vectorize(fieldNames: string[]) {
    const json = {
      index_name: this.indexName,
      field_names: fieldNames,
    }

    return await ky
      .post(`${this.url}/index/vectorize`, {
        headers: this.headers,
        json: json,
      })
      .json()
  }
}

export { Index }
