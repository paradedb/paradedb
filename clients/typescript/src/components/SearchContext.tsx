"use client"
import { createContext } from "react"

interface SearchContextProps {
  apiKey: string
  url: string
}

const SearchContext = createContext<SearchContextProps | undefined>(undefined)

export default SearchContext
