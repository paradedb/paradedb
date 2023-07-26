from fastapi import APIRouter, status
from opensearchpy import helpers
from pydantic import BaseModel
from starlette.responses import JSONResponse
from typing import List, Optional

from core.search.client import Client
from core.extract.postgres import PostgresExtractor

router = APIRouter()
tag = "client"

# TODO: Replace hard-coded credentials and add SSL
client = Client(
    host="core", port=9200, user="admin", password="admin", verify_certs=False
)


class IndexPayload(BaseModel):
    source_host: str
    source_port: int
    source_user: str
    source_password: str
    source_relation: str
    source_primary_key: str
    source_columns: List[str]
    source_neural_columns: List[str] = []
    reindex: Optional[bool] = False


@router.post(f"/{tag}/index", tags=[tag])
async def index(payload: IndexPayload) -> JSONResponse:
    if not payload.source_columns:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST, content=f"source_columns is empty"
        )

    if not set(payload.source_neural_columns).issubset(set(payload.source_columns)):
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"source_neural_columns must be a subset of source_columns",
        )

    if payload.reindex:
        client.delete_index(payload.source_relation)

    # Create index
    index = None
    try:
        index = client.create_index(payload.source_relation)
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Relation {payload.source_relation} is already indexed. If you want to re-index {payload.source_relation}, set reindex=True.",
        )

    # Register neural search fields
    index.register_neural_search_fields(payload.source_neural_columns)

    # Number of rows to extract at once
    BATCH_SIZE = 500

    try:
        extractor = PostgresExtractor(
            host=payload.source_host,
            port=payload.source_port,
            user=payload.source_user,
            password=payload.source_password,
        )

        for chunk in extractor.extract_all(
            relation=payload.source_relation,
            columns=payload.source_columns,
            primary_key=payload.source_primary_key,
            chunk_size=BATCH_SIZE,
        ):
            rows = chunk.get("rows")
            primary_keys = chunk.get("primary_keys")

            if rows and primary_keys:
                index.upsert(documents=rows, ids=primary_keys)

        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"{payload.source_relation} indexed successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to index data: {e}",
        )
