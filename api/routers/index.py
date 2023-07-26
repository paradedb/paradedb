from fastapi import APIRouter, status
from opensearchpy import helpers
from pydantic import BaseModel
from starlette.responses import JSONResponse
from typing import List, Dict, Any

from core.search.client import Client
from core.search.index import default_model_name

router = APIRouter()
tag = "index"

# TODO: Replace hard-coded credentials and add SSL
client = Client(
    host="core", port=9200, user="admin", password="admin", verify_certs=False
)


class IndexCreatePayload(BaseModel):
    name: str


class RegisterNeuralSearchFieldsPayload(BaseModel):
    index_name: str
    fields: List[str]


class InsertDocumentsPayload(BaseModel):
    index_name: str
    documents: List[Dict[str, Any]]


class SearchPayload(BaseModel):
    index_name: str
    dsl: Dict[str, Any]


@router.post(f"/{tag}/create", tags=[tag])
async def create_index(payload: IndexCreatePayload) -> JSONResponse:
    try:
        client.create_index(payload.name)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Index {payload.name} created successfully",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to create index {payload.name}: {e}",
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


@router.post(f"/{tag}/insert", tags=[tag])
async def insert_documents(payload: InsertDocumentsPayload) -> JSONResponse:
    try:
        index = client.get_index(payload.index_name)
        index.insert(payload.documents)
        return JSONResponse(
            status_code=status.HTTP_200_OK,
            content=f"Documents inserted successfully into {payload.index_name}",
        )
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to insert documents: {e}",
        )


@router.post(f"/{tag}/search", tags=[tag])
async def search_documents(payload: SearchPayload) -> JSONResponse:
    try:

        def add_model_id(nested_dict, model_id):
            for key, value in nested_dict.items():
                if isinstance(value, dict):
                    if "source" not in value.keys():
                        add_model_id(value, model_id)
                    if key == "neural":
                        for inner_key, inner_value in value.items():
                            if (
                                isinstance(inner_value, dict)
                                and "source" not in inner_value.keys()
                            ):
                                inner_value["model_id"] = model_id
                elif isinstance(value, list):
                    for item in value:
                        if isinstance(item, dict):
                            add_model_id(item, model_id)

        index = client.get_index(payload.index_name)
        dsl = payload.dsl
        model = index.model.get(default_model_name)

        if not model:
            return JSONResponse(
                status_code=status.HTTP_400_BAD_REQUEST,
                content=f"Table {payload.index_name} is not properly registered. Did you forget to index() the table?",
            )

        model_id = model["model_id"]
        add_model_id(dsl, model_id)
        print(dsl)

        return JSONResponse(status_code=status.HTTP_200_OK, content=index.search(dsl))
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=f"Failed to search documents: {e}",
        )
