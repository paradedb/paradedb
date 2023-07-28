from opensearchpy import OpenSearch
from opensearchpy.exceptions import NotFoundError
from typing import Dict, Union, Any, cast


class Pipeline:
    def __init__(self, client: OpenSearch) -> None:
        self.client = client

    def create(self, pipeline_id: str) -> None:
        self.client.ingest.put_pipeline(id=pipeline_id, body={"processors": []})

    def get(self, pipeline_id: str) -> Union[Dict[str, Any], None]:
        try:
            response = self.client.ingest.get_pipeline(id=pipeline_id)
            return cast(Dict[str, Any], response)
        except NotFoundError:
            return None

    def create_processor(self, pipeline_id: str, processor: Dict[str, Any]) -> None:
        self.client.ingest.put_pipeline(
            id=pipeline_id, body={"processors": [processor]}
        )
