import helpers from "opensearch-js"
import { useState, useEffect, useContext, useRef } from "react"

import { Client } from "../lib/client"
import { Index } from "../lib/index"
import SearchContext from "./SearchContext"

interface SearchOptions {
  indexName: string
  query: helpers.RequestBodySearch
}

const useSearch = ({ indexName, query }: SearchOptions) => {
  const context = useContext(SearchContext)
  const [data, setData] = useState<Record<string, any>>()
  const [error, setError] = useState<string>()
  const [index, setIndex] = useState<Index>()

  if (!context?.apiKey || !context?.url) {
    throw new Error("apiKey and url must be set in SearchProvider")
  }

  const client = new Client(context.apiKey, context.url)
  const prevQueryRef = useRef<string>()
  const jsonQuery = query.toJSON()

  useEffect(() => {
    client
      .getIndex(indexName)
      .then((index) => setIndex(index))
      .catch((err) => setError(err))
  }, [])

  useEffect(() => {
    const stringifiedQuery = JSON.stringify(jsonQuery)

    if (prevQueryRef.current !== stringifiedQuery && index !== undefined) {
      index
        .search(query)
        .then((res: any) => setData(res))
        .catch((err: string) => setError(err))

      prevQueryRef.current = stringifiedQuery
    }
  }, [indexName, jsonQuery])

  return { data, error }
}

export default useSearch
