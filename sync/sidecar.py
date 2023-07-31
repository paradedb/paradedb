from starlette.applications import Starlette
from starlette.responses import JSONResponse
from starlette.routing import Route
from loguru import logger


async def bootstrap(request):
    body = await request.json()
    logger.info(body)


app = Starlette(
    debug=True,
    routes=[
        Route("/bootstrap", bootstrap, methods=["POST"]),
    ],
)
