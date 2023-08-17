from opensearchpy import AsyncOpenSearch
from opensearchpy.exceptions import NotFoundError
from typing import Dict, Union, Any, cast


class Pipeline:
    def __init__(self, client: AsyncOpenSearch) -> None:
        self.client = client

    async def create(self, pipeline_id: str) -> None:
        await self.client.ingest.put_pipeline(id=pipeline_id, body={"processors": []})

    async def get(self, pipeline_id: str) -> Union[Dict[str, Any], None]:
        try:
            response = await self.client.ingest.get_pipeline(id=pipeline_id)
            return cast(Dict[str, Any], response)
        except NotFoundError:
            return None

    async def create_processor(
        self, pipeline_id: str, processor: Dict[str, Any]
    ) -> None:
        await self.client.ingest.put_pipeline(
            id=pipeline_id, body={"processors": [processor]}
        )
