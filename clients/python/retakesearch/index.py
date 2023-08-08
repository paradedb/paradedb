import httpx

from opensearchpy import Search
from typing import Any, List, Dict, Optional, Union


class Database:
    def __init__(self, host: str, user: str, password: str, port: int, dbname: str):
        self.host = host
        self.user = user
        self.password = password
        self.port = port
        self.dbname = dbname


class Table:
    def __init__(
        self,
        name: str,
        columns: List[str],
        schema: str = "public",
        transform: Optional[Dict[str, Any]] = None,
        relationship: Optional[Dict[str, Any]] = None,
        children: Optional[List["Table"]] = None,
    ):
        self.name = name
        self.columns = columns
        self.transform = transform
        self.schema = schema
        self.relationship = relationship
        self.children = children

    def to_schema(self) -> Dict[str, Any]:
        schema: Dict[str, Any] = {
            "table": self.name,
            "columns": self.columns,
            "schema": self.schema,
        }

        if self.transform:
            schema["transform"] = self.transform

        if self.relationship:
            schema["relationship"] = self.relationship

        if self.children:
            schema["children"] = [child.to_schema() for child in self.children]

        return schema


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
        source = {
            "index_name": self.index_name,
            "source_host": database.host,
            "source_user": database.user,
            "source_password": database.password,
            "source_port": database.port,
            "source_dbname": database.dbname,
        }

        pgsync_schema: Dict[str, Any] = dict()
        pgsync_schema["database"] = database.dbname
        pgsync_schema["index"] = self.index_name
        pgsync_schema["nodes"] = table.to_schema()

        json = {
            "source": source,
            "pgsync_schema": pgsync_schema,
        }

        print(
            f"Preparing to sync index {self.index_name} with table {table.name}. This may take some time if your table is large..."
        )

        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/add_source", headers=self.headers, json=json
            )
            if not response.status_code == 200:
                raise Exception(response.text)

    def search(self, search: Search) -> Any:
        json = {
            "dsl": search.to_dict(),  # type: ignore
            "index_name": self.index_name,
        }

        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/search", headers=self.headers, json=json
            )
            if response.status_code == 200:
                return response.json()
            else:
                raise Exception(response.text)

    def upsert(
        self, documents: List[Dict[str, Any]], ids: List[Union[str, int]]
    ) -> Any:
        json = {"index_name": self.index_name, "documents": documents, "ids": ids}

        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/upsert", headers=self.headers, json=json
            )
            if response.status_code == 200:
                return response.json()
            else:
                raise Exception(response.text)

    def create_field(
        self,
        field_name: str,
        field_type: str,
        dimension: Optional[int] = None,
        space_type: Optional[str] = None,
        engine: Optional[str] = None,
    ) -> None:
        json = {
            "index_name": self.index_name,
            "field_name": field_name,
            "field_type": field_type,
            "dimension": dimension,
            "space_type": space_type,
            "engine": engine,
        }

        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/field/create", headers=self.headers, json=json
            )
            if not response.status_code == 200:
                raise Exception(response.text)

    def vectorize(
        self,
        field_names: List[str],
        space_type: Optional[str] = None,
        engine: Optional[str] = None,
    ) -> None:
        json = {
            "index_name": self.index_name,
            "field_names": field_names,
            "space_type": space_type,
            "engine": engine,
        }

        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/vectorize", headers=self.headers, json=json
            )
            if not response.status_code == 200:
                raise Exception(response.text)
