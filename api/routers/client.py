from fastapi import APIRouter, status
from starlette.responses import JSONResponse

from api.config.opensearch import OpenSearchConfig
from api.config.pgsync import PgSyncConfig

from core.search.client import Client

tag = "client"

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


@router.get("/{tag}/indices", tags=[tag])
async def get_indices() -> JSONResponse:
    try:
        indices = await client.list_indices()
        return JSONResponse(status_code=status.HTTP_200_OK, content=indices)
    except Exception as e:
        return JSONResponse(
            status_code=status.HTTP_400_BAD_REQUEST,
            content=str(e),
        )
