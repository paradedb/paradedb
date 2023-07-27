import httpx

from opensearchpy import Search
from typing import Any, List


class Database:
    def __init__(self, host: str, user: str, password: str, port: int):
        self.host = host
        self.user = user
        self.password = password
        self.port = port


class Table:
    def __init__(
        self, name: str, primary_key: str, columns: List[str], neural_columns: List[str]
    ):
        self.name = name
        self.primary_key = primary_key
        self.columns = columns
        self.neural_columns = neural_columns


class Index:
    def __init__(self, index_name: str, api_key: str, url: str) -> None:
        self.index_name = index_name
        self.api_key = api_key
        self.url = url

        self.headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

    def add_source(self, database: Database, table: Table) -> Any:
        json = {
            "index_name": self.index_name,
            "source_host": database.host,
            "source_user": database.user,
            "source_password": database.password,
            "source_port": database.port,
            "source_relation": table.name,
            "source_primary_key": table.primary_key,
            "source_columns": table.columns,
            "source_neural_columns": table.neural_columns,
        }

        try:
            with httpx.Client(timeout=None) as http:
                print(
                    f"Adding {table.name} to index {self.index_name}. This could take some time if the table is large..."
                )
                response = http.post(
                    f"{self.url}/index/add_source", headers=self.headers, json=json
                )
                response.raise_for_status()
                return response.json()
        except httpx.HTTPStatusError as exc:
            return exc.response.json()
        except Exception as exc:
            return str(exc)

    def search(self, search: Search) -> Any:
        json = {
            "dsl": search.to_dict(),  # type: ignore
            "index_name": self.index_name,
        }
        try:
            with httpx.Client(timeout=None) as http:
                response = http.post(
                    f"{self.url}/index/search", headers=self.headers, json=json
                )
                response.raise_for_status()
                return response.json()
        except httpx.HTTPStatusError as exc:
            return exc.response.json()
        except Exception as exc:
            return str(exc)
