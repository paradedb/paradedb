import psycopg2
import select
import json
import requests
import threading
import time
import queue

from http import HTTPStatus
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from typing import List, Generator, Dict, Any, Optional, cast

from core.extract.base import Extractor, ExtractorResult
from core.extract.realtime import create_connector


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
                host=self.host, user=self.user, password=self.password, port=self.port
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

            # Convert rows into list of dicts, excluding primary keys
            rows = [dict(zip(columns, row[:-1])) for row in rows]

            yield {"rows": rows, "primary_keys": primary_keys}
            offset += chunk_size

    def extract_real_time(
        self,
        connect_server: str,
        schema_registry_server: str,
        relation: str,
        primary_key: str,
        columns: List[str],
    ) -> None:
        include_columns = [f"{self.schema_name}.{relation}.{col}" for col in columns]

        # Always include the primary key so it can be extracted for indexing
        include_columns.append(f"{self.schema_name}.{relation}.{primary_key}")

        connector_config = {
            "name": f"{relation}-connector",
            "config": {
                "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
                "plugin.name": "pgoutput",
                "value.converter": "io.confluent.connect.avro.AvroConverter",
                "value.converter.schema.registry.url": schema_registry_server,
                "database.hostname": f"{self.host}",
                "database.port": f"{self.port}",
                "database.user": f"{self.user}",
                "database.password": f"{self.password}",
                "database.dbname": f"{self.dbname}",
                "table.include.list": f"{self.schema_name}.{relation}",
                "column.include.list": ",".join(include_columns),
                "slot.name": f"debezium_{relation}",
                "transforms": "unwrap",
                "transforms.unwrap.type": "io.debezium.transforms.ExtractNewRecordState",
                "transforms.unwrap.drop.tombstones": "false",
                "transforms.unwrap.delete.handling.mode": "rewrite",
                "topic.prefix": f"{relation}",
            },
        }

        try:
            create_connector(connect_server, connector_config)
        except requests.exceptions.HTTPError as e:
            print("Connector already exists")
        except requests.exceptions.RequestException as e:
            print(e)

    def is_connector_ready(self, connect_server: str, relation: str) -> bool:
        connector_name = f"{relation}-connector"
        url = f"{connect_server}/connectors/{connector_name}"
        timeout = 15

        start_time = time.time()

        while time.time() - start_time < timeout:
            response = requests.get(url)
            if response.status_code == HTTPStatus.OK:
                return True
            time.sleep(1)

        return False
