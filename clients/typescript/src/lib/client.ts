import ky from "ky"
import { Index } from "./index"

class Client {
  private apiKey: string
  private url: string

  constructor(apiKey: string, url: string) {
    this.apiKey = apiKey
    this.url = url
  }

  async getIndex(indexName: string) {
    const response = await ky.get(`${this.url}/index/${indexName}`, {
      headers: {
        Authorization: `Bearer ${this.apiKey}`,
      },
    })

    if (response.ok) {
      return new Index(indexName, this.apiKey, this.url)
    } else {
      const text = await response.text()
      throw new Error(text)
    }
  }

  async createIndex(indexName: string) {
    const response = await ky.post(`${this.url}/index/create`, {
      headers: {
        Authorization: `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      json: { index_name: indexName },
    })

    if (response.ok) {
      return new Index(indexName, this.apiKey, this.url)
    } else {
      const text = await response.text()
      throw new Error(text)
    }
  }

  async deleteIndex(indexName: string) {
    const response = await ky.post(`${this.url}/index/delete`, {
      headers: {
        Authorization: `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      json: { index_name: indexName },
    })

    if (!response.ok) {
      const text = await response.text()
      throw new Error(text)
    }
  }
}

export { Client }
