import httpx
from opensearchpy import Search
from typing import Any, List

from .index import Index


class Client:
    def __init__(self, api_key: str, url: str):
        self.api_key = api_key
        self.url = url

    def get_index(self, index_name: str) -> Index:
        with httpx.Client(timeout=None) as http:
            response = http.get(
                f"{self.url}/index/{index_name}",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                },
            )
            response.raise_for_status()
            return Index(index_name=index_name, api_key=self.api_key)

    def create_index(self, index_name: str) -> Index:
        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/create",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                },
                json={"name": index_name},
            )
            response.raise_for_status()
            return Index(index_name=index_name, api_key=self.api_key)
