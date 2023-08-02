import requests
from fastapi import APIRouter, status
from loguru import logger
from opensearchpy.exceptions import RequestError
from pydantic import BaseModel
from starlette.responses import JSONResponse
from typing import List, Dict, Any, Optional, Union

from api.config.opensearch import OpenSearchConfig
from api.config.pgsync import PgSyncConfig

from core.extract.postgres import PostgresExtractor
from core.search.client import Client
from core.search.index import default_model_name
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
    verify_certs=opensearch_config.use_tls,
)


class IndexCreatePayload(BaseModel):
    index_name: str


class IndexDeletePayload(BaseModel):
    index_name: str


class VectorizePayload(BaseModel):
    index_name: str
    field_names: List[str]


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
        client.get_index(index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK, content=f"Index {index_name} found"
        )
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
        index = client.get_index(payload.index_name)
        index.upsert(payload.documents, payload.ids)
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
        client.create_index(payload.index_name)
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
        client.delete_index(payload.index_name)
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
        source = {k: str(v) if not isinstance(v, str) else v for k, v in payload.source.model_dump().items()}
        body = {"source": source, "schema": [payload.pgsync_schema]}

        logger.info(body)

        logger.info(f"Preparing to send sync request to {pgsync_config.url}")
        res = requests.post(f"{pgsync_config.url}/sync", json=body)
        logger.info(f"Got sync response {res.content}")

        if res.status_code == status.HTTP_200_OK:
            return JSONResponse(
                status_code=status.HTTP_200_OK,
                content="Real time sync started successfully",
            )
        else:
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Could not start real time sync: {res.content}",
            )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=str(e),
        )


@router.post(f"/{tag}/search", tags=[tag])
async def search_documents(payload: SearchPayload) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK, content=index.search(payload.dsl)
        )
    except RequestError as e:
        not_vectorized_error_stub = "is not knn_vector type"
        if not_vectorized_error_stub in str(e):
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

        index = client.get_index(payload.index_name)
        index.mappings.upsert(
            properties={payload.field_name: {"type": payload.field_type}}
        )
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
async def vectorize_field(payload: VectorizePayload) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)
        index.register_neural_search_fields(payload.field_names)
        # Reindexing is necessary to generate vectors for existing document fields
        index.reindex()
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Fields {payload.field_names} vectorized successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to vectorize fields: {e}",
        )
