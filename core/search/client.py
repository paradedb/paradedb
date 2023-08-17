from opensearchpy import AsyncOpenSearch

from core.search.index import Index
from typing import List

# Users are not allowed to create indices with these prefixes
# because these are reserved for internal use by OpenSearch
reserved_index_prefixes = [".", "security-auditlog"]


class Client:
    def __init__(
        self, host: str, port: int, user: str, password: str, verify_certs: bool = True
    ) -> None:
        self.client = AsyncOpenSearch(
            hosts=[{"host": host, "port": port}],
            http_compress=True,
            http_auth=(user, password),
            use_ssl=True,
            verify_certs=verify_certs,
            max_retries=3,
            retry_on_timeout=True,
            timeout=60 * 30,
        )

    # Private Methods

    async def _check_index_exists(self, index_name: str) -> bool:
        return await self.client.indices.exists(index=index_name)

    # Public Methods

    async def create_index(self, index_name: str) -> Index:
        if await self._check_index_exists(index_name=index_name):
            raise ValueError(f"Index {index_name} already exists")

        if any(index_name.startswith(prefix) for prefix in reserved_index_prefixes):
            raise ValueError(
                f"Index {index_name} cannot start with any of {reserved_index_prefixes}"
            )

        await self.client.indices.create(index=index_name)

        return Index(name=index_name, client=self.client)

    async def get_index(self, index_name: str) -> Index:
        if not await self._check_index_exists(index_name=index_name):
            raise ValueError(f"Index {index_name} does not exist")

        return Index(name=index_name, client=self.client)

    async def delete_index(self, index_name: str) -> None:
        await self.client.indices.delete(index=index_name, ignore=[400, 404])

    async def list_indices(self) -> List[str]:
        indices = await self.client.indices.get_alias()
        index_names = filter(
            lambda x: not any(
                x.startswith(prefix) for prefix in reserved_index_prefixes
            ),
            indices.keys(),
        )
        return list(index_names)
