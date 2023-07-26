import React from "react"
import SearchContext from "./SearchContext"

interface SearchProviderProps {
  apiKey: string
  url: string
  children: React.ReactNode
}

const SearchProvider: React.FC<SearchProviderProps> = ({
  apiKey,
  url,
  children,
}) => {
  return (
    <SearchContext.Provider value={{ apiKey, url }}>
      {children}
    </SearchContext.Provider>
  )
}

export default SearchProvider
