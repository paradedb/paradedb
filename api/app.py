import os
import urllib3
import logging
import uuid

from fastapi import FastAPI, Request, status
from starlette.middleware.cors import CORSMiddleware
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.responses import JSONResponse
from typing import Callable, Optional, Any
from posthog import Posthog

from .routers import index
from .routers import client
from .routers import base

# TODO: Add SSL and remove this
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
logging.basicConfig(level=logging.INFO)

API_KEY = os.getenv("API_KEY", "")
COMMIT_SHA = os.getenv("COMMIT_SHA", "")
POSTHOG_API_KEY = os.getenv("POSTHOG_API_KEY", "")
TELEMETRY = os.getenv("TELEMETRY", "enabled")


if POSTHOG_API_KEY != "" and TELEMETRY != "disabled":
    logging.info("Telemetry enabled")
    posthog = Posthog(project_api_key=POSTHOG_API_KEY, host="https://app.posthog.com")

    # Keep all telemetry as anonymous
    posthog.capture(
        str(uuid.uuid4()), "New Retake Deployment", {"Commit SHA": COMMIT_SHA}
    )
else:
    logging.info("Telemetry disabled")


class APIKeyValidator:
    def __init__(self, api_key: str) -> None:
        self.api_key = api_key

    def __call__(self, request: Request) -> Optional[JSONResponse]:
        if request.method == "OPTIONS":
            return None

        if "Authorization" not in request.headers:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content="Missing Authorization Header",
            )
        try:
            bearer = request.headers["Authorization"]
            scheme, token = bearer.strip().split(" ")
            if not all([scheme.lower() == "bearer", token == self.api_key]):
                return JSONResponse(
                    status_code=status.HTTP_401_UNAUTHORIZED,
                    content="Invalid API Key",
                )
            return None
        except Exception:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content="Invalid API Key",
            )


class AuthMiddleware(BaseHTTPMiddleware):
    def __init__(self, app: FastAPI, api_key_validator: APIKeyValidator) -> None:
        super().__init__(app)
        self.api_key_validator = api_key_validator

    async def dispatch(self, request: Request, call_next: Callable[..., Any]) -> Any:
        auth_response = self.api_key_validator(request)
        if auth_response is not None and auth_response.status_code != 200:
            return auth_response
        response = await call_next(request)
        return response


app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_credentials=True,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)
app.add_middleware(AuthMiddleware, api_key_validator=APIKeyValidator(API_KEY))
app.include_router(index.router)
app.include_router(base.router)
app.include_router(client.router)
