import psycopg2

from typing import List, Generator, cast

from core.extract.base import Extractor, ExtractorResult


class ConnectionError(Exception):
    pass


class PostgresExtractor(Extractor):
    def __init__(
        self,
        host: str,
        user: str,
        password: str,
        port: int,
        schema_name: str = "public",
        dbname: str = "postgres",
    ) -> None:
        self.host = host
        self.user = user
        self.password = password
        self.port = port
        self.schema_name = schema_name
        self.dbname = dbname
        self._connect()

    def _connect(self) -> None:
        try:
            self.connection = psycopg2.connect(
                host=self.host,
                user=self.user,
                password=self.password,
                port=self.port,
                dbname=self.dbname,
            )
        except psycopg2.ProgrammingError as e:
            raise ConnectionError(
                f"Unable to connect to database {self.dbname} {self.host} {self.port} {self.user} {self.password}: {e}"
            )
        except psycopg2.OperationalError as e:
            raise ConnectionError(
                f"Unable to connect to database {self.dbname} {self.host} {self.port} {self.user} {self.password}: {e}"
            )

        self.cursor = self.connection.cursor()

    def validate(self, relation: str, columns: List[str], primary_key: str) -> None:
        # Check if the relation (table) exists
        self.cursor.execute(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = %s AND table_name = %s);",
            (self.schema_name, relation),
        )
        relation_exists = self.cursor.fetchone()[0]  # type: ignore
        if not relation_exists:
            raise ValueError(
                f"The relation '{relation}' does not exist in the schema '{self.schema_name}'"
            )

        # Check if the columns are valid
        self.cursor.execute(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = %s AND table_name = %s;",
            (self.schema_name, relation),
        )
        actual_columns = [row[0] for row in self.cursor.fetchall()]
        for col in columns:
            if col not in actual_columns:
                raise ValueError(
                    f"The column '{col}' does not exist in relation '{relation}'"
                )

        if primary_key not in actual_columns:
            raise ValueError(
                f"The primary key '{primary_key}' does not exist in relation '{relation}'"
            )

    def teardown(self) -> None:
        self.cursor.close()  # type: ignore
        self.connection.close()

    def count(self, relation: str) -> int:
        self.cursor.execute(f"SELECT COUNT(*) FROM {relation}")
        row = self.cursor.fetchone()
        if row:
            return cast(int, row[0])
        else:
            return 0

    def extract_all(
        self, relation: str, columns: List[str], primary_key: str, chunk_size: int
    ) -> Generator[ExtractorResult, None, None]:
        offset = 0
        columns_str = ", ".join(columns)

        while True:
            self.cursor.execute(
                f"""
                SELECT {columns_str}, {primary_key}
                FROM {relation}
                ORDER BY {primary_key}
                LIMIT %s
                OFFSET %s
            """,
                (chunk_size, offset),
            )

            rows = self.cursor.fetchall()

            if not rows:
                break

            # Extract primary keys from rows
            primary_keys = [row[-1] for row in rows]

            # Convert rows into list of dicts, excluding primary keys
            output = [dict(zip(columns, row[:-1])) for row in rows]

            yield {"rows": output, "primary_keys": primary_keys}
            offset += chunk_size
