import psycopg2
import select
import json
import threading
import queue

from psycopg2.extras import LogicalReplicationConnection
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT


class ConnectionError(Exception):
    pass


class PostgresExtractor:
    def __init__(self, dsn: str):
        self.dsn = dsn
        self.connection = None
        self.cursor = None
        self.chunk_size = 1000

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

    def disconnect(self):
        self.connection.close()

    def fetch_rows(self, table, columns):
        offset = 0
        columns_str = ", ".join(columns)

        while True:
            self.cursor.execute(
                f"""
                SELECT {columns_str}
                FROM {table}
                ORDER BY id
                LIMIT %s
                OFFSET %s
            """,
                (self.chunk_size, offset),
            )

            rows = self.cursor.fetchall()

            if not rows:
                break

            yield rows
            offset += self.chunk_size
