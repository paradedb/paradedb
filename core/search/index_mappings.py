from opensearchpy import OpenSearch
from typing import Dict, Any


class IndexMappings:
    def __init__(self, name: str, client: OpenSearch):
        self.name = name
        self.client = client

    def upsert(self, properties: Dict[str, Any]) -> None:
        # We upsert one-by-one, so if a single property fails, it does not
        # affect the others
        for attribute, values in properties.items():
            try:
                body = {"properties": {attribute: values}}
                self.client.indices.put_mapping(index=self.name, body=body)
            except Exception as e:
                print(f"Failed to upsert {attribute} with {values}: {e}")
