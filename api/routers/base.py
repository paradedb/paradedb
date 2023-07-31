from fastapi import APIRouter, status
from starlette.responses import JSONResponse

router = APIRouter()


@router.get("/")
async def get() -> JSONResponse:
    return JSONResponse(status_code=status.HTTP_200_OK, content=None)
