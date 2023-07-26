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
import { helpers, request, useSearch } from "retake-search";
const { multiMatchQuery } = helpers;

export default () => {
  const [userInput, setUserInput] = useState<string>("");
  const table = process.env.DATABASE_TABLE_NAME ?? "";
  const columns = process.env.DATABASE_TABLE_COLUMNS
    ? JSON.parse(process.env.DATABASE_TABLE_COLUMNS)
    : [];
  const query = request.query(multiMatchQuery(columns, userInput)).toJSON();

  const { data, error } = useSearch({ table, query });
  const results = data?.hits?.hits;

  if (error) {
    return (
      <Card>
        <Text>{error}</Text>
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
