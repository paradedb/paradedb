import { useState } from "react";
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
} from "@tremor/react";
import { Search, useSearch } from "retake-search";
import { withNeural } from "retake-search/helpers";

export default () => {
  const [userInput, setUserInput] = useState<string>("");
  const index = process.env.DATABASE_TABLE_NAME ?? "";
  const columns = process.env.DATABASE_TABLE_COLUMNS
    ? JSON.parse(process.env.DATABASE_TABLE_COLUMNS)
    : [];
  const query = Search().query(withNeural(userInput, columns));

  const { data, error } = useSearch({ indexName: index, query: query });
  const results = data?.hits?.hits;

  if (error) {
    return (
      <Card>
        <Text>An unexpected error occured: {error}</Text>
      </Card>
    );
  }

  return (
    <Card>
      <TextInput
        value={userInput}
        onChange={(event) => setUserInput(event.target.value)}
      />
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
    </Card>
  );
};
