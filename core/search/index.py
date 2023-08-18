import time
import json

from enum import Enum
from loguru import logger
from opensearchpy import AsyncOpenSearch, helpers
from typing import Dict, List, Any, Union, AsyncGenerator, cast

from core.search.index_mappings import IndexMappings, FieldType
from core.search.index_settings import IndexSettings
from core.search.model_group import ModelGroup
from core.search.model import Model
from core.search.pipeline import Pipeline


default_model_group = "default_model_group"
default_model_name = "huggingface/sentence-transformers/all-MiniLM-L12-v2"
default_model_version = "1.0.1"
default_model_format = "TORCH_SCRIPT"
default_engine = "faiss"
default_algorithm = "hnsw"
default_space_type = "l2"
default_model_dimensions = 384
reserved_embedding_field_name_ending = "_retake_embedding"


class TaskStatus(Enum):
    FAILED = "FAILED"
    COMPLETED = "COMPLETED"


class OpenSearchTaskException(Exception):
    pass


class ModelNotLoadedException(Exception):
    pass


class Index:
    def __init__(self, name: str, client: AsyncOpenSearch) -> None:
        self.name = name
        self.client = client
        self.settings = IndexSettings(name, client)
        self.mappings = IndexMappings(name, client)
        self.model_group = ModelGroup(client)
        self.model = Model(client)
        self.pipeline = Pipeline(client)
        self.pipeline_id = f"{self.name}_pipeline"

    # Private Methods

    async def _wait_for_task_result(self, task_id: str) -> Dict[str, Any]:
        task_status = None
        response = None
        wait_time_seconds = 2

        while task_status not in [TaskStatus.COMPLETED.value, TaskStatus.FAILED.value]:
            response = await self.client.transport.perform_request(
                "GET", f"/_plugins/_ml/tasks/{task_id}"
            )

            logger.info(response)

            task_status = response["state"]  # type: ignore

            if task_status == TaskStatus.FAILED:
                raise OpenSearchTaskException(json.dumps(response))

            time.sleep(wait_time_seconds)

        return cast(Dict[str, Any], response)

    async def _get_embedding_field_names(self) -> List[str]:
        properties = (await self.client.indices.get_mapping(index=self.name))[
            self.name
        ]["mappings"].get("properties", dict())

        knn_vector_properties = []

        for prop, prop_data in properties.items():
            if prop_data.get("type") == FieldType.KNN_VECTOR.value:
                knn_vector_properties.append(prop)

        return knn_vector_properties

    async def _load_model(self, model_name: str) -> str:
        logger.info("Loading model...")

        model_group = await self.model_group.get(default_model_group)
        logger.info("Loaded model group")

        if not model_group:
            logger.info("Model group not found, creating")
            model_group = await self.model_group.create(default_model_group)
            logger.info("Created model group")

        model_group_id = model_group["model_group_id"]

        logger.info("Registering model")

        # Get/register model
        model = await self.model.get(model_name)

        logger.info("Waiting for task result")
        if not model:
            response = await self.model.register(
                name=model_name,
                version=default_model_version,
                model_format=default_model_format,
                model_group_id=model_group_id,
            )
            task_id = response["task_id"]
            task_result = await self._wait_for_task_result(task_id)
            model_id = task_result.get("model_id", None)

            if not model_id:
                raise Exception(task_result)
        else:
            model_id = model.get("model_id", None)

        logger.info(f"Loading and deploying model: {model_id}")
        resp = await self.model.load(model_id)
        await self._wait_for_task_result(resp["task_id"])

        logger.info("Model loaded")

        resp = await self.model.deploy(model_id)
        await self._wait_for_task_result(resp["task_id"])

        logger.info(f"Model deployed: {resp}")

        return cast(str, model_id)

    # Public Methods
    async def upsert(
        self, documents: List[Dict[str, Any]], ids: List[Union[str, int]]
    ) -> None:
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
        await helpers.async_bulk(self.client, formatted_documents)
        logger.info(f"Successfully bulk upserted {len(formatted_documents)} documents")

    async def search(self, dsl: Dict[str, Any]) -> Dict[str, Any]:
        def add_model_id(nested_dict: Dict[str, Any], model_id: str) -> None:
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

        pipeline = await self.pipeline.get(pipeline_id=self.pipeline_id)

        if pipeline:
            model_id = (
                pipeline.get(self.pipeline_id, dict())
                .get("processors", [dict()])[0]
                .get("text_embedding", dict())
                .get("model_id")
            )
            add_model_id(dsl, model_id)

        # Get embedding field names
        embedding_field_names = await self._get_embedding_field_names()

        if "_source" in dsl and isinstance(dsl["_source"], dict):
            dsl["_source"]["excludes"] = embedding_field_names
        else:
            dsl["_source"] = {"excludes": embedding_field_names}

        return cast(Dict[str, Any], await self.client.search(index=self.name, body=dsl))

    async def register_neural_search_fields(
        self,
        fields: List[str],
        space_type: str,
        engine: str,
        model_name: str,
        model_dimension: int,
    ) -> None:
        logger.info("Registering neural search fields")

        # Get/create model
        model_id = await self._load_model(model_name)
        logger.info(f"Loaded model {model_id}")

        # Get/create pipeline
        pipeline = await self.pipeline.get(pipeline_id=self.pipeline_id)
        logger.info("Loaded pipeline")

        if not pipeline:
            logger.info("Pipeline not found, creating")
            await self.pipeline.create(pipeline_id=self.pipeline_id)

        # Update index settings to use pipeline
        logger.info("Updating index settings")
        await self.settings.update(
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

        logger.info("Creating processor")
        await self.pipeline.create_processor(
            pipeline_id=self.pipeline_id,
            processor=processor,
        )

        # Update index settings to use new neural search fields
        logger.info("Upserting new fields")
        await self.mappings.upsert(
            properties={
                f"{field}{reserved_embedding_field_name_ending}": {
                    "type": FieldType.KNN_VECTOR.value,
                    "dimension": model_dimension,
                    "method": {
                        "name": default_algorithm,
                        "engine": engine,
                        "space_type": space_type,
                    },
                }
                for field in fields
            }
        )

    async def reindex(self, fields: List[str]) -> None:
        logger.info("Reindexing fields")

        async def _generator() -> AsyncGenerator[Dict[str, Any], None]:
            """Generator function to fetch all documents from the specified index."""
            async for hit in helpers.async_scan(self.client, index=self.name):
                # Note: cast explicitly here since mypy infers the opensearch-py
                # generator to be AsyncGenerator[int, None]
                h = cast(Dict[str, Any], hit)
                doc = {k: h["_source"][k] for k in fields if k in h["_source"]}

                if not doc:
                    continue

                yield {
                    "_op_type": "update",
                    "_index": h["_index"],
                    "_id": h["_id"],
                    "doc": {k: h["_source"][k] for k in fields if k in h["_source"]},
                    "doc_as_upsert": True,
                }

        await helpers.async_bulk(
            self.client,
            _generator(),
        )

    async def describe(self) -> Dict[str, Any]:
        count = (await self.client.count(index=self.name))["count"]
        mapping = await self.client.indices.get_mapping(index=self.name)
        all_properties = (
            mapping.get(self.name, dict())
            .get("mappings", dict())
            .get("properties", dict())
        )
        fields = {
            k: v
            for k, v in all_properties.items()
            if not k.endswith(reserved_embedding_field_name_ending)
        }
        vectorized_fields = {
            k[: -len(reserved_embedding_field_name_ending)]: v
            for k, v in all_properties.items()
            if k.endswith(reserved_embedding_field_name_ending)
        }

        return {
            "count": count,
            "fields": fields,
            "vectorized_fields": vectorized_fields,
        }
