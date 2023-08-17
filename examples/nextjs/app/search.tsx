import { useState } from "react"
import {
  TextInput,
  Card,
  Table,
  TableHead,
  TableHeaderCell,
  TableBody,
  TableRow,
  TableCell,
  Text,
  Flex,
  Button,
  Metric,
} from "@tremor/react"
import { Search } from "retake-search"
import { useSearch } from "retake-search/components"
import { withNeural } from "retake-search/helpers"

const index = process.env.DATABASE_TABLE_NAME ?? ""
const columns = process.env.DATABASE_TABLE_COLUMNS
  ? JSON.parse(process.env.DATABASE_TABLE_COLUMNS)
  : []

const SearchComponent = () => {
  const [userInput, setUserInput] = useState<string>("")
  const [searchQuery, setSearchQuery] = useState<string>("")

  const query = Search().query(withNeural(searchQuery, columns))
  const { data, error } = useSearch({ indexName: index, query: query })
  const results = data?.hits?.hits

  if (error) {
    return (
      <Card>
        <Text>An unexpected error occured: {error.toString()}</Text>
      </Card>
    )
  }

  return (
    <Card>
      <Metric>Search Demo</Metric>
      <Flex className="gap-x-4 mt-6">
        <TextInput
          value={userInput}
          onChange={event => setUserInput(event.target.value)}
          placeholder="Search your dataset here"
          onKeyDown={evt => {
            if (evt.key === "Enter") setSearchQuery(userInput)
          }}
        />
        <Button onClick={() => setSearchQuery(userInput)} color="indigo">
          Search
        </Button>
      </Flex>
      {searchQuery === "" ? (
        <Flex className="mt-5 justify-center">
          <Text className="mt-4">Start typing to see search results</Text>
        </Flex>
      ) : (
        <Table className="mt-5">
          <TableHead>
            <TableRow>
              {columns.map((column: string, index: number) => (
                <TableHeaderCell key={index}>{column}</TableHeaderCell>
              ))}
            </TableRow>
          </TableHead>
          <TableBody>
            {results?.map((result: any, index: number) => (
              <TableRow key={index}>
                {columns.map((column: string, index: number) => (
                  <TableCell key={index}>{result?._source?.[column]}</TableCell>
                ))}
              </TableRow>
            )) ?? <></>}
          </TableBody>
        </Table>
      )}
    </Card>
  )
}

export default SearchComponent
