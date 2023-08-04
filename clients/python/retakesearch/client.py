import httpx

from typing import Optional, List, Dict, Any, cast

from .index import Index


class Client:
    def __init__(self, api_key: str, url: str):
        self.api_key = api_key
        self.url = url

    def get_index(self, index_name: str) -> Optional[Index]:
        with httpx.Client(timeout=None) as http:
            response = http.get(
                f"{self.url}/index/{index_name}",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                },
            )
            if response.status_code == 200:
                return Index(index_name=index_name, api_key=self.api_key, url=self.url)
            else:
                raise Exception(response.text)

    def create_index(self, index_name: str) -> Optional[Index]:
        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/create",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                },
                json={"index_name": index_name},
            )
            if response.status_code == 200:
                return Index(index_name=index_name, api_key=self.api_key, url=self.url)
            else:
                raise Exception(response.text)

    def delete_index(self, index_name: str) -> None:
        with httpx.Client(timeout=None) as http:
            response = http.post(
                f"{self.url}/index/delete",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                },
                json={"index_name": index_name},
            )
            if not response.status_code == 200:
                raise Exception(response.text)

    def list_indices(self) -> List[str]:
        with httpx.Client(timeout=None) as http:
            response = http.get(
                f"{self.url}/client/indices",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                },
            )
            if response.status_code == 200:
                return cast(List[str], response.json())
            else:
                raise Exception(response.text)

    def describe_index(self, index_name: str) -> Dict[str, Any]:
        with httpx.Client(timeout=None) as http:
            response = http.get(
                f"{self.url}/index/{index_name}",
                headers={"Authorization": f"Bearer {self.api_key}"},
            )
            if response.status_code == 200:
                return cast(Dict[str, Any], response.json())
            else:
                raise Exception(response.text)
