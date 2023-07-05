import psycopg2
import select
import json
import threading
import queue

from psycopg2.extras import LogicalReplicationConnection
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from typing import List

from core.extract.base import Extractor


class ConnectionError(Exception):
    pass


class PostgresExtractor(Extractor):
    def __init__(self, dsn: str):
        self.dsn = dsn
        self.connection = None
        self.cursor = None

        self._connect(dsn)

    def _connect(self, dsn: str):
        try:
            self.connection = psycopg2.connect(
                self.dsn, connection_factory=LogicalReplicationConnection
            )
        except psycopg2.ProgrammingError:
            raise ConnectionError("Unable to connect to database")
        except psycopg2.OperationalError:
            raise ConnectionError("Unable to connect to database")

        self.cursor = self.connection.cursor()

    def teardown(self):
        self.cursor.close()
        self.connection.close()

    def count(self, relation: str):
        self.cursor.execute(f"SELECT COUNT(*) FROM {relation}")
        return self.cursor.fetchone()[0]

    def extract_all(
        self, relation: str, columns: List[str], primary_key: str, chunk_size: int
    ):
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
