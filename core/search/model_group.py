from opensearchpy import AsyncOpenSearch
from opensearchpy.exceptions import NotFoundError
from typing import Dict, Union, Any, cast


class ModelGroup:
    def __init__(self, client: AsyncOpenSearch) -> None:
        self.client = client

    async def get(self, name: str) -> Union[Dict[str, Any], None]:
        request_body = {
            "query": {"term": {"name.keyword": name}},
        }

        try:
            response = await self.client.transport.perform_request(
                "POST",
                "/_plugins/_ml/model_groups/_search",
                body=request_body,
            )

            if response["hits"]["total"]["value"] == 0:  # type: ignore
                return None

            model_group = response["hits"]["hits"][0]  # type: ignore
            model_group["model_group_id"] = model_group["_id"]
            return cast(Dict[str, Any], model_group)
        except NotFoundError:
            return None

    async def create(self, name: str, access_mode: str = "public") -> Dict[str, Any]:
        request_body = {"name": name, "model_access_mode": access_mode}

        # Send the request
        response = await self.client.transport.perform_request(
            "POST", "/_plugins/_ml/model_groups/_register", body=request_body
        )

        return cast(Dict[str, Any], response)
