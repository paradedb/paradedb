import httpx
from fastapi import APIRouter, status
from loguru import logger
from opensearchpy.exceptions import RequestError
from pydantic import BaseModel
from starlette.background import BackgroundTask
from starlette.responses import JSONResponse
from typing import List, Dict, Any, Optional, Union, cast

from api.config.opensearch import OpenSearchConfig
from api.config.pgsync import PgSyncConfig

from core.search.index import (
    default_algorithm,
    default_engine,
    default_space_type,
    default_model_name,
    default_model_dimensions,
)
from core.search.client import Client
from core.search.index_mappings import FieldType

tag = "index"

router = APIRouter()
opensearch_config = OpenSearchConfig()
pgsync_config = PgSyncConfig()

client = Client(
    host=opensearch_config.host,
    port=opensearch_config.port,
    user=opensearch_config.user,
    password=opensearch_config.password,
    verify_certs=opensearch_config.verify_certs,
)
aclient = httpx.AsyncClient()


class IndexCreatePayload(BaseModel):
    index_name: str


class IndexDeletePayload(BaseModel):
    index_name: str


class VectorizePayload(BaseModel):
    index_name: str
    field_names: List[str]
    engine: Optional[str] = default_engine
    space_type: Optional[str] = default_space_type
    model_name: Optional[str] = default_model_name
    model_dimension: Optional[int] = default_model_dimensions


class SearchPayload(BaseModel):
    index_name: str
    dsl: Dict[str, Any]


class UpsertPayload(BaseModel):
    index_name: str
    documents: List[Dict[str, Any]]
    ids: List[Union[str, int]]


class CreateFieldPayload(BaseModel):
    index_name: str
    field_name: str
    field_type: str
    dimension: Optional[int] = default_model_dimensions
    space_type: Optional[str] = default_space_type
    engine: Optional[str] = default_engine


class Database(BaseModel):
    index_name: str
    source_host: str
    source_port: int
    source_user: str
    source_password: str
    source_dbname: str = "postgres"
    source_schema_name: str = "public"


class AddSourcePayload(BaseModel):
    source: Database
    pgsync_schema: Dict[str, Any]


@router.get("/index/{index_name}", tags=[tag])
async def get_index(index_name: str) -> JSONResponse:
    try:
        index = await client.get_index(index_name)
        description = await index.describe()
        return JSONResponse(status_code=status.HTTP_200_OK, content=description)
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=str(e),
        )


@router.post("/{tag}/upsert", tags=[tag])
async def upsert(payload: UpsertPayload) -> JSONResponse:
    try:
        if not len(payload.documents) == len(payload.ids):
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content="Length of documents and ids arrays must be equal",
            )
        index = await client.get_index(payload.index_name)
        await index.upsert(payload.documents, payload.ids)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content="Documents upserted successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to upsert documents: {e}",
        )


@router.post(f"/{tag}/create", tags=[tag])
async def create_index(payload: IndexCreatePayload) -> JSONResponse:
    try:
        await client.create_index(payload.index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Index {payload.index_name} created successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to create index {payload.index_name}: {e}",
        )


@router.post(f"/{tag}/delete", tags=[tag])
async def delete_index(payload: IndexDeletePayload) -> JSONResponse:
    try:
        await client.delete_index(payload.index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Index {payload.index_name} deleted successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to delete index {payload.index_name}: {e}",
        )


@router.post(f"/{tag}/add_source", tags=[tag])
async def add_source(payload: AddSourcePayload) -> JSONResponse:
    try:
        source = {
            k: str(v) if not isinstance(v, str) else v
            for k, v in payload.source.model_dump().items()
        }
        body = {"source": source, "schema": [payload.pgsync_schema]}

        logger.info(body)
        logger.info(f"Preparing to send sync request to {pgsync_config.url}")

        res = await aclient.post(f"{pgsync_config.url}/sync", json=body, timeout=None)
        logger.info(f"Got sync response {res.text}")

        if res.status_code == status.HTTP_200_OK:
            return JSONResponse(
                status_code=status.HTTP_200_OK,
                content="Real time sync started successfully",
                background=BackgroundTask(res.aclose),
            )
        else:
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Could not start real time sync: {res.text}",
                background=BackgroundTask(res.aclose),
            )
    except Exception as e:
        logger.error(e)
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=str(e),
        )


@router.post(f"/{tag}/search", tags=[tag])
async def search_documents(payload: SearchPayload) -> JSONResponse:
    try:
        index = await client.get_index(payload.index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK, content=await index.search(payload.dsl)
        )
    except RequestError as e:
        not_vectorized_errors = [
            "is not knn_vector type",
            "Model not ready yet",
            "[bool] failed to parse field [should]",
        ]

        if any([err in str(e) for err in not_vectorized_errors]):
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Failed to search index {payload.index_name} because not all fields were vectorized. Did you call Index.vectorize()?",
            )
        else:
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Failed to search index {payload.index_name}: {e}",
            )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to search index {payload.index_name}: {e}",
        )


@router.post(f"/{tag}/field/create", tags=[tag])
async def create_field(payload: CreateFieldPayload) -> JSONResponse:
    try:
        field_types = [e.value for e in FieldType]
        if payload.field_type not in field_types:
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Invalid field type: {payload.field_type}. Accepted values are {field_types}",
            )

        index = await client.get_index(payload.index_name)

        # Set index.knn = True
        await index.settings.update(settings={"index.knn": True})

        # Update index mapping
        properties: Dict[str, Any] = {payload.field_name: {"type": payload.field_type}}

        if payload.field_type == FieldType.KNN_VECTOR.value:
            properties[payload.field_name]["dimension"] = cast(
                int, payload.model_dump().get("dimension")
            )
            properties[payload.field_name]["method"] = {
                "name": default_algorithm,
                "space_type": cast(str, payload.model_dump().get("space_type")),
                "engine": cast(str, payload.model_dump().get("engine")),
            }

        await index.mappings.upsert(properties=properties)

        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Field {payload.field_name} created successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to create field: {e}",
        )


@router.post(f"/{tag}/vectorize", tags=[tag])
async def vectorize(payload: VectorizePayload) -> JSONResponse:
    try:
        index = await client.get_index(payload.index_name)
        await index.register_neural_search_fields(
            payload.field_names,
            engine=cast(str, payload.model_dump().get("engine")),
            space_type=cast(str, payload.model_dump().get("space_type")),
            model_name=cast(str, payload.model_dump().get("model_name")),
            model_dimension=cast(int, payload.model_dump().get("model_dimension")),
        )
        # Reindexing is necessary to generate vectors for existing document fields
        logger.info("Neural search fields registered. Reindexing...")
        await index.reindex(payload.field_names)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Fields {payload.field_names} vectorized successfully",
        )
    except Exception as e:
        logger.error(e)
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to vectorize fields: {e}",
        )
