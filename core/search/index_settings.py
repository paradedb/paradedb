from opensearchpy import AsyncOpenSearch
from typing import Dict, Any


class IndexSettings:
    def __init__(self, name: str, client: AsyncOpenSearch):
        self.name = name
        self.client = client

    async def update(self, settings: Dict[str, Any]) -> None:
        # Close the index, update the settings, and reopen the index
        await self.client.indices.close(index=self.name)
        await self.client.indices.put_settings(index=self.name, body=settings)
        await self.client.indices.open(index=self.name)
