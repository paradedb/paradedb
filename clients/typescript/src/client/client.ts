import ky from "ky"

type Database = {
  host: string
  user: string
  password: string
  port: number
}

type Table = {
  name: string
  primaryKey: string
  columns: string[]
}

class Client {
  private apiKey: string
  private url: string

  constructor({ apiKey, url }: { apiKey: string; url: string }) {
    this.apiKey = apiKey
    this.url = url
  }

  public index = async ({
    database,
    table,
    reindex = false,
  }: {
    database: Database
    table: Table
    reindex: boolean
  }): Promise<any> => {
    return await ky
      .post(`${this.url}/client/index`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        json: {
          source_host: database.host,
          source_user: database.user,
          source_password: database.password,
          source_port: database.port,
          source_relation: table.name,
          source_primary_key: table.primaryKey,
          source_columns: table.columns,
          reindex,
        },
        timeout: false,
      })
      .json()
      .catch(async (error: any) => {
        if (error.name === "HTTPError") {
          return error.response.json()
        } else {
          return error
        }
      })
  }

  public search = async ({
    table,
    dsl,
  }: {
    table: string
    dsl: Record<string, any>
  }) => {
    return await ky
      .post(`${this.url}/index/search`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        json: {
          dsl,
          index_name: table,
        },
        timeout: false,
      })
      .json()
      .catch(async (error: any) => {
        if (error.name === "HTTPError") {
          return error.response.json()
        } else {
          return error
        }
      })
  }
}

export { Client }
