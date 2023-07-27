import time

from enum import Enum
from pydantic import BaseModel
from opensearchpy import OpenSearch, Search, helpers
from opensearchpy.exceptions import NotFoundError
from typing import Dict, List, Optional, Any, Union, cast

from core.search.index_mappings import IndexMappings
from core.search.index_settings import IndexSettings
from core.search.model_group import ModelGroup
from core.search.model import Model
from core.search.pipeline import Pipeline

# TODO: Allow these values to be updated by the user
# Hard-coded as defaults for now
default_model_group = "default_model_group"
default_model_name = "huggingface/sentence-transformers/all-MiniLM-L12-v2"
default_model_version = "1.0.1"
default_model_format = "TORCH_SCRIPT"
default_model_dimensions = 384

reserved_embedding_field_name_ending = "_retake_embedding"


class TaskStatus(Enum):
    FAILED = "FAILED"
    COMPLETED = "COMPLETED"


class OpenSearchTaskException(Exception):
    pass


class Index:
    def __init__(self, name: str, client: OpenSearch) -> None:
        self.name = name
        self.client = client
        self.settings = IndexSettings(name, client)
        self.mappings = IndexMappings(name, client)
        self.model_group = ModelGroup(client)
        self.model = Model(client)
        self.pipeline = Pipeline(client)
        self.pipeline_id = f"{self.name}_pipeline"

    ### Private Methods ###

    def _wait_for_task_result(self, task_id: str) -> Dict[str, Any]:
        task_status = None
        response = None
        wait_time_seconds = 2

        while not task_status == TaskStatus.COMPLETED.value:
            response = self.client.transport.perform_request(
                "GET", f"/_plugins/_ml/tasks/{task_id}"
            )
            task_status = response["state"]  # type: ignore

            if task_status == TaskStatus.FAILED:
                raise OpenSearchTaskException(response["error"])  # type: ignore

            time.sleep(wait_time_seconds)

        return cast(Dict[str, Any], response)

    def _get_embedding_field_names(self) -> List[str]:
        properties = self.client.indices.get_mapping(index=self.name)[self.name][
            "mappings"
        ]["properties"]
        knn_vector_properties = []

        for prop, prop_data in properties.items():
            if prop_data.get("type") == "knn_vector":
                knn_vector_properties.append(prop)

        return knn_vector_properties

    ### Public Methods ###
    def upsert(self, documents: List[Dict[str, Any]], ids: List[str]) -> None:
        formatted_documents = [
            {
                "_op_type": "update",
                "_index": self.name,
                "_id": _id,
                "doc": document,
                "doc_as_upsert": True,
            }
            for document, _id in zip(documents, ids)
        ]
        helpers.bulk(self.client, formatted_documents)

    def search(self, dsl: Dict[str, Any]) -> Dict[str, Any]:
        def add_model_id(nested_dict, model_id):
            for key, value in nested_dict.items():
                if isinstance(value, dict):
                    if "source" not in value.keys():
                        add_model_id(value, model_id)
                    if key == "neural":
                        for _, inner_value in value.items():
                            if (
                                isinstance(inner_value, dict)
                                and "source" not in inner_value.keys()
                            ):
                                inner_value["model_id"] = model_id
                elif isinstance(value, list):
                    for item in value:
                        if isinstance(item, dict):
                            add_model_id(item, model_id)

        model = self.model.get(default_model_name)

        if not model:
            raise Exception(f"Model {default_model_name} not found.")

        model_id = model["model_id"]
        add_model_id(dsl, model_id)

        # Get embedding field names
        embedding_field_names = self._get_embedding_field_names()

        if "_source" in dsl and isinstance(dsl["_source"], dict):
            dsl["_source"]["excludes"] = embedding_field_names
        else:
            dsl["_source"] = {"excludes": embedding_field_names}

        return cast(Dict[str, Any], self.client.search(index=self.name, body=dsl))

    def register_neural_search_fields(self, fields: Optional[List[str]] = None) -> None:
        if fields:
            # Get/create model group
            model_group = self.model_group.get(default_model_group)
            if not model_group:
                model_group = self.model_group.create(default_model_group)

            model_group_id = model_group["model_group_id"]

            # Get/register model
            model = self.model.get(default_model_name)
            model_id = None

            if not model:
                response = self.model.register(
                    name=default_model_name,
                    version=default_model_version,
                    model_format=default_model_format,
                    model_group_id=model_group_id,
                )
                task_id = response["task_id"]
                task_result = self._wait_for_task_result(task_id)
                model_id = task_result["model_id"]
            else:
                model_id = model["model_id"]

            # Load the model
            response = self.model.load(model_id)
            self._wait_for_task_result(response["task_id"])

            # Get/create pipeline
            pipeline = self.pipeline.get(pipeline_id=self.pipeline_id)

            if not pipeline:
                self.pipeline.create(pipeline_id=self.pipeline_id)

            # Update index settings to use pipeline
            self.settings.update(
                settings={"index.knn": True, "default_pipeline": self.pipeline_id}
            )

            # Add new neural search fields to pipeline
            processor = {
                "text_embedding": {
                    "model_id": model_id,
                    "field_map": {
                        field: f"{field}{reserved_embedding_field_name_ending}"
                        for field in fields
                    },
                }
            }

            self.pipeline.create_processor(
                pipeline_id=self.pipeline_id,
                processor=processor,
            )

            # Update index settings to use new neural search fields
            self.mappings.upsert(
                properties={
                    f"{field}{reserved_embedding_field_name_ending}": {
                        "type": "knn_vector",
                        "dimension": default_model_dimensions,
                        "method": {"name": "hnsw", "engine": "lucene"},
                    }
                    for field in fields
                }
            )
