from fastapi import APIRouter, BackgroundTasks, status
from loguru import logger
from pydantic import BaseModel
from starlette.responses import JSONResponse
from typing import List, Dict, Any, Union

from api.config.kafka import KafkaConfig
from api.config.opensearch import OpenSearchConfig
from api.sink_consumer import consume_records

from core.extract.postgres import PostgresExtractor, ConnectionError
from core.search.client import Client

router = APIRouter()
tag = "index"

opensearch_config = OpenSearchConfig()

client = Client(
    host=opensearch_config.host,
    port=opensearch_config.port,
    user=opensearch_config.user,
    password=opensearch_config.password,
    verify_certs=opensearch_config.use_tls,
)


class IndexCreatePayload(BaseModel):
    index_name: str


class RegisterNeuralSearchFieldsPayload(BaseModel):
    index_name: str
    fields: List[str]


class SearchPayload(BaseModel):
    index_name: str
    dsl: Dict[str, Any]


class UpsertPayload(BaseModel):
    index_name: str
    documents: List[Dict[str, Any]]
    ids: List[Union[str, int]]


class AddSourcePayload(BaseModel):
    index_name: str
    source_host: str
    source_port: int
    source_user: str
    source_password: str
    source_relation: str
    source_primary_key: str
    source_columns: List[str]
    source_neural_columns: List[str] = []
    source_dbname: str = "postgres"
    source_schema_name: str = "public"


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
async def upsert(payload: UpsertPayload):
    try:
        if not len(payload.documents) == len(payload.ids):
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Length of documents and ids arrays must be equal",
            )
        index = client.get_index(payload.index_name)
        index.upsert(payload.documents, payload.ids)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Documents upserted successfully",
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


@router.post(f"/{tag}/add_source", tags=[tag])
async def add_source(payload: AddSourcePayload) -> JSONResponse:
    # Number of rows to extract at once
    BATCH_SIZE = 500

    try:
        index = client.get_index(payload.index_name)

        extractor = PostgresExtractor(
            host=payload.source_host,
            port=payload.source_port,
            user=payload.source_user,
            password=payload.source_password,
        )

        if payload.source_neural_columns:
            index.register_neural_search_fields(payload.source_neural_columns)

        for chunk in extractor.extract_all(
            relation=payload.source_relation,
            columns=payload.source_columns,
            primary_key=payload.source_primary_key,
            chunk_size=BATCH_SIZE,
        ):
            rows = chunk.get("rows")
            primary_keys = chunk.get("primary_keys")

            if rows and primary_keys:
                print("upserting", rows)
                index.upsert(documents=rows, ids=primary_keys)

        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Source {payload.source_relation} linked to index {payload.index_name} successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=str(e),
        )


@router.post(f"/{tag}/realtime/link", tags=[tag])
async def realtime_link(payload: AddSourcePayload) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)

        extractor = PostgresExtractor(
            host=payload.source_host,
            port=payload.source_port,
            user=payload.source_user,
            password=payload.source_password,
            dbname=payload.source_dbname,
            schema_name=payload.source_schema_name,
        )
        logger.info("Successfully setup extractor")

        kafka_config = KafkaConfig()

        if payload.source_neural_columns:
            logger.info("Registering neural search fields...")
            index.register_neural_search_fields(payload.source_neural_columns)
            logger.info("Successfully registered neural search fields")

        extractor.extract_real_time(
            kafka_config.connect_server,
            kafka_config.schema_registry_server,
            payload.source_relation,
            payload.source_primary_key,
            payload.source_columns,
        )
        logger.info("Created connector. Waiting for it to be ready...")

        if extractor.is_connector_ready(
            kafka_config.connect_server, payload.source_relation
        ):
            logger.info("Connector ready!")
            return JSONResponse(
                status_code=status.HTTP_200_OK,
                content=f"{payload.source_relation} linked successfully",
            )
        else:
            return JSONResponse(
                status_code=status.HTTP_400_OK,
                content=f"Failed to link data. Connector not created successfully. Check the Kafka Connect logs for more information",
            )
    except ConnectionError as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Could not connect to database: {e}",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to link data: {e}",
        )


@router.post(f"/{tag}/realtime/start", tags=[tag])
async def realtime_start(
    payload: AddSourcePayload, bg_tasks: BackgroundTasks
) -> JSONResponse:
    logger.info("Starting consumer coroutine...")

    # Create index
    index = None
    try:
        index = client.get_index(payload.source_relation)
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Index {payload.source_relation} not found",
        )

    # Follow topic naming convention used by Debezium
    topic = f"{payload.source_relation}.{payload.source_schema_name}.{payload.source_relation}"
    bg_tasks.add_task(consume_records, topic, payload.source_primary_key, index.upsert)

    return JSONResponse(
        status_code=status.HTTP_200_OK,
        content=f"Processing started on index {payload.source_relation}",
    )


@router.post(f"/{tag}/register_neural_search_fields", tags=[tag])
async def register_neural_search_fields(
    payload: RegisterNeuralSearchFieldsPayload,
) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)
        index.register_neural_search_fields(payload.fields)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Neural search fields for {payload.index_name} registered successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to register neural search fields: {e}",
        )


@router.post(f"/{tag}/search", tags=[tag])
async def search_documents(payload: SearchPayload) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)
        return JSONResponse(
            status_code=status.HTTP_200_OK, content=index.search(payload.dsl)
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to search documents: {e}",
        )
