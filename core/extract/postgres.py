import psycopg2

from psycopg2.extras import LogicalReplicationConnection
from typing import List, Generator, cast

from core.extract.base import Extractor, ExtractorResult


class ConnectionError(Exception):
    pass


class PostgresExtractor(Extractor):
    def __init__(self, dsn: str) -> None:
        self.dsn = dsn
        self._connect(dsn)

    def _connect(self, dsn: str) -> None:
        try:
            self.connection = psycopg2.connect(
                self.dsn, connection_factory=LogicalReplicationConnection
            )
        except psycopg2.ProgrammingError:
            raise ConnectionError("Unable to connect to database")
        except psycopg2.OperationalError:
            raise ConnectionError("Unable to connect to database")

        self.cursor = self.connection.cursor()

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

            # Remove primary keys from rows
            rows = [row[:-1] for row in rows]

            yield {"rows": rows, "primary_keys": primary_keys}
            offset += chunk_size
