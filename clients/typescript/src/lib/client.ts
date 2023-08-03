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
    return await ky
      .get(`${this.url}/index/${indexName}`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
        },
      })
      .then((response) => {
        if (response.ok) return new Index(indexName, this.apiKey, this.url)
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }

  async createIndex(indexName: string) {
    return await ky
      .post(`${this.url}/index/create`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        json: { index_name: indexName },
        timeout: false,
      })
      .then((response) => {
        if (response.ok) return new Index(indexName, this.apiKey, this.url)
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }

  async deleteIndex(indexName: string) {
    await ky
      .post(`${this.url}/index/delete`, {
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        json: { index_name: indexName },
        timeout: false,
      })
      .catch(async (err) => {
        throw new Error(await err.response.text())
      })
  }
}

export { Client }
