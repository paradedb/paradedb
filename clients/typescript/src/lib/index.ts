import ky from "ky"
import helpers from "opensearch-js"

interface TableSchema {
  table: string
  columns: string[]
  transform?: { [key: string]: any }
  relationship?: { [key: string]: any }
  children?: TableSchema[]
}

interface DatabaseSchema {
  host: string
  user: string
  password: string
  port: number
  dbName: string
}

class Database {
  host: string
  user: string
  password: string
  port: number
  dbName: string

  constructor(args: DatabaseSchema) {
    this.host = args.host
    this.user = args.user
    this.password = args.password
    this.port = args.port
    this.dbName = args.dbName
  }
}

class Table {
  table: string
  columns: string[]
  transform?: { [key: string]: any }
  relationship?: { [key: string]: any }
  children?: Table[]

  constructor(args: TableSchema) {
    this.table = args.table
    this.columns = args.columns
    this.transform = args.transform
    this.relationship = args.relationship

    if (args.children)
      this.children = args.children.map((child) => new Table(child))
  }

  toSchema(): TableSchema {
    const schema: TableSchema = {
      table: this.table,
      columns: this.columns,
    }

    if (this.transform) schema.transform = this.transform
    if (this.relationship) schema.relationship = this.relationship
    if (this.children)
      schema.children = this.children.map((child) => child.toSchema())

    return schema
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
    const source = {
      index_name: this.indexName,
      source_host: database.host,
      source_user: database.user,
      source_password: database.password,
      source_port: database.port,
      source_dbname: database.dbName,
    }

    const pgsyncSchema = {
      database: database.dbName,
      index: this.indexName,
      nodes: table.toSchema(),
    }

    const json = {
      source,
      pgsync_schema: pgsyncSchema,
    }

    console.log(
      `Preparing to sync index ${this.indexName} with table ${table.table}. This may take some time if your table is large...`
    )

    await ky
      .post(`${this.url}/index/add_source`, {
        headers: this.headers,
        json: json,
        timeout: false,
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
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
        timeout: false,
      })
      .then((response) => response.json())
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }

  async upsert(
    documents: Record<string, any>[],
    ids: (string | number)[]
  ): Promise<any> {
    const json = { index_name: this.indexName, documents: documents, ids: ids }

    await ky
      .post(`${this.url}/index/upsert`, {
        headers: this.headers,
        json: json,
        timeout: false,
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }

  async createField(fieldName: string, fieldType: string) {
    const json = {
      index_name: this.indexName,
      field_name: fieldName,
      field_type: fieldType,
    }

    await ky
      .post(`${this.url}/index/field/create`, {
        headers: this.headers,
        json: json,
        timeout: false,
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }

  async vectorize(fieldNames: string[]) {
    const json = {
      index_name: this.indexName,
      field_names: fieldNames,
    }

    await ky
      .post(`${this.url}/index/vectorize`, {
        headers: this.headers,
        json: json,
        timeout: false,
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }
}

export { Index, Database, Table, DatabaseSchema, TableSchema }
