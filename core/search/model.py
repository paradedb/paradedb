from opensearchpy import AsyncOpenSearch
from opensearchpy.exceptions import NotFoundError
from typing import Dict, Union, Any, cast


class Model:
    def __init__(self, client: AsyncOpenSearch):
        self.client = client

    async def get(self, name: str) -> Union[Dict[str, Any], None]:
        request_body = {
            "query": {"term": {"name.keyword": name}},
        }

        try:
            response = await self.client.transport.perform_request(
                "POST",
                "/_plugins/_ml/models/_search",
                body=request_body,
            )

            if response["hits"]["total"]["value"] == 0:  # type: ignore
                return None

            model = response["hits"]["hits"][0]  # type: ignore
            source = model["_source"]

            return cast(Dict[str, Any], source)
        except NotFoundError:
            return None

    async def register(
        self, name: str, version: str, model_format: str, model_group_id: str
    ) -> Dict[str, Any]:
        request_body = {
            "name": name,
            "version": version,
            "model_format": model_format,
            "model_group_id": model_group_id,
        }

        # Send the request
        response = await self.client.transport.perform_request(
            "POST", "/_plugins/_ml/models/_upload", body=request_body
        )
        return cast(Dict[str, Any], response)

    async def load(self, model_id: str) -> Dict[str, Any]:
        response = await self.client.transport.perform_request(
            "POST", f"/_plugins/_ml/models/{model_id}/_load"
        )
        return cast(Dict[str, Any], response)

    async def deploy(self, model_id: str) -> Dict[str, Any]:
        response = await self.client.transport.perform_request(
            "POST", f"/_plugins/_ml/models/{model_id}/_deploy"
        )
        return cast(Dict[str, Any], response)
