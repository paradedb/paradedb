import { useState, useEffect, useContext, useRef } from "react"
import { Client } from "../client"
import SearchContext from "./SearchContext"

interface SearchOptions {
  table: string
  query: Record<string, any>
}

const useSearch = ({ table, query }: SearchOptions) => {
  const context = useContext(SearchContext)
  const [data, setData] = useState<null | Record<string, any>>(null)
  const [error, setError] = useState(null)

  if (!context?.apiKey || !context?.url) {
    throw new Error("apiKey and url must be set in SearchProvider")
  }

  const client = new Client({
    apiKey: context.apiKey,
    url: context.url,
  })

  const prevQueryRef = useRef<string>()

  useEffect(() => {
    const stringifiedQuery = JSON.stringify(query)

    if (prevQueryRef.current !== stringifiedQuery) {
      client
        .search({ table, dsl: query })
        .then((res: any) => setData(res))
        .catch((err) => setError(err))

      prevQueryRef.current = stringifiedQuery
    }
  }, [table, query])

  return { data, error }
}

export default useSearch
